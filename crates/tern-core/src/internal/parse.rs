// TODO(qcoh): Redo this whole thing with `nom`.
use super::SqlDialect;

pub(super) fn split_sql(
    input: &str,
    dialect: Option<SqlDialect>,
) -> Vec<String> {
    input
        .split_inclusive('\n')
        .fold(Splitter::new(dialect), Splitter::line)
        .finish()
}

enum Escaped {
    Normal(State),
    // `'...'`; `escapes` when a backslash escapes the next character (MySQL,
    // or a postgres `E'...'` string).  Standard `''` doubling needs no
    // special case: it closes and immediately reopens the string.
    SingleQuote { escapes: bool },
    // `"..."`; MySQL also honors backslash escapes here.
    DoubleQuote,
    // MySQL backtick-quoted identifier.
    Backtick,
    // Sqlite `[...]`-quoted identifier.
    Bracket,
    // Postgres dollar quote; holds the full `$tag$` closer.
    DollarQuote(String),
    // Depth of `/* ... */`; only postgres allows them to nest.
    BlockComment(usize),
}

#[derive(Clone, Copy)]
enum State {
    Plain,
    // Saw `END`: a following `IF`/`LOOP`/`WHILE`/`REPEAT`/`CASE` closes
    // with it as a unit and must not read as a new opener.
    AfterEnd,
    // Saw `IF` inside a block: it opens an `IF ... THEN ... END IF` block
    // unless what follows is `(`, `NOT`, or `EXISTS`.
    AfterIf,
}

pub(super) struct Splitter {
    dialect: Option<SqlDialect>,
    state: Escaped,
    depth: usize,
    buf: String,
    stmts: Vec<String>,
}

impl Splitter {
    fn new(dialect: Option<SqlDialect>) -> Self {
        Self {
            dialect,
            state: Escaped::Normal(State::Plain),
            depth: 0,
            buf: String::new(),
            stmts: Vec::new(),
        }
    }

    // Flush whatever trails the last `;`.
    fn finish(mut self) -> Vec<String> {
        push_stmt(&mut self.stmts, &self.buf);
        self.stmts
    }

    // Process a line, then we fold this over the lines iterator, accumulating
    // state changes and ending the loop if in normal, non-escaped state and
    // a term character is found.
    fn line(mut self, line: &str) -> Self {
        let mut rest = line;
        while !rest.is_empty() {
            let state = std::mem::replace(
                &mut self.state,
                Escaped::Normal(State::Plain),
            );
            rest = match state {
                Escaped::Normal(ctx) => self.normal(rest, ctx),
                Escaped::SingleQuote { escapes } => {
                    self.string(rest, '\'', escapes)
                },
                Escaped::DoubleQuote => self.string(
                    rest,
                    '"',
                    self.dialect.is_some_and(|d| d == SqlDialect::MySql),
                ),
                Escaped::Backtick => self.string(rest, '`', false),
                Escaped::Bracket => self.string(rest, ']', false),
                Escaped::DollarQuote(closer) => self.until(rest, closer),
                Escaped::BlockComment(depth) => self.block_comment(rest, depth),
            };
        }
        self
    }

    fn is_mysql(&self) -> bool {
        self.dialect.is_some_and(|d| d == SqlDialect::MySql)
    }

    fn is_psql(&self) -> bool {
        self.dialect.is_some_and(|d| d == SqlDialect::Postgres)
    }

    fn is_sqlite(&self) -> bool {
        self.dialect.is_some_and(|d| d == SqlDialect::Sqlite)
    }

    // In statement position: everything up to the next context-changing
    // character is plain SQL whose words feed the block-depth tracker.
    fn normal<'a>(&mut self, rest: &'a str, ctx: State) -> &'a str {
        const SPECIAL: &[char] =
            &['\'', '"', '`', '[', '$', ';', '#', '-', '/'];
        let (span, at) = match rest.find(SPECIAL) {
            Some(pos) => rest.split_at(pos),
            None => (rest, ""),
        };
        let ctx = self.track_words(ctx, span);
        self.buf.push_str(span);
        if at.is_empty() {
            self.state = Escaped::Normal(ctx);
            return "";
        }

        self.state = Escaped::Normal(State::Plain);

        let mysql = self.is_mysql();
        if at.starts_with("--") || (mysql && at.starts_with('#')) {
            self.buf.push_str(at);
            return "";
        }
        if let Some(after) = at.strip_prefix("/*") {
            self.buf.push_str("/*");
            return self.block_comment(after, 1);
        }
        if let Some(after) = at.strip_prefix(';') {
            self.buf.push(';');
            if self.depth == 0 {
                push_stmt(&mut self.stmts, &self.buf);
                self.buf.clear();
            }
            return after;
        }
        if let Some(after) = at.strip_prefix('\'') {
            let escapes =
                mysql || (self.is_psql() && estring_prefix(&self.buf));
            self.buf.push('\'');
            self.state = Escaped::SingleQuote { escapes };
            return after;
        }
        if let Some(after) = at.strip_prefix('"') {
            self.buf.push('"');
            self.state = Escaped::DoubleQuote;
            return after;
        }
        if mysql && let Some(after) = at.strip_prefix('`') {
            self.buf.push('`');
            self.state = Escaped::Backtick;
            return after;
        }
        if self.is_sqlite()
            && let Some(after) = at.strip_prefix('[')
        {
            self.buf.push('[');
            self.state = Escaped::Bracket;
            return after;
        }
        // A `$` opens a dollar quote only in postgres and only at a word
        // boundary, so an identifier like `foo$bar` doesn't start one.
        if self.is_psql()
            && at.starts_with('$')
            && !self.buf.chars().next_back().is_some_and(is_word_char)
            && let Some((closer, after)) = dollar_opener(at)
        {
            self.buf.push_str(&closer);
            self.state = Escaped::DollarQuote(closer);
            return after;
        }
        // A lone `-`, `/`, `$`, or a quote character foreign to this
        // dialect: plain text.
        let mut chars = at.chars();
        self.buf.push(chars.next().expect("`at` is non-empty"));
        chars.as_str()
    }

    // Copy text through the closing delimiter of a quoted region, honoring
    // backslash escapes; an unclosed delimiter carries the state into the
    // next line.
    fn string<'a>(
        &mut self,
        rest: &'a str,
        close: char,
        escapes: bool,
    ) -> &'a str {
        match rest.split_once(close) {
            None => {
                self.buf.push_str(rest);
                self.state = quote_state(close, escapes);
                ""
            },
            Some((body, after)) => {
                self.buf.push_str(body);
                self.buf.push(close);
                if escapes && escaped(body) {
                    self.string(after, close, escapes)
                } else {
                    self.state = Escaped::Normal(State::Plain);
                    after
                }
            },
        }
    }

    // Copy text through `closer` (a `$tag$`); tags are word characters, so
    // a closer can never straddle a line boundary.
    fn until<'a>(&mut self, rest: &'a str, closer: String) -> &'a str {
        match rest.split_once(closer.as_str()) {
            None => {
                self.buf.push_str(rest);
                self.state = Escaped::DollarQuote(closer);
                ""
            },
            Some((body, after)) => {
                self.buf.push_str(body);
                self.buf.push_str(&closer);
                self.state = Escaped::Normal(State::Plain);
                after
            },
        }
    }

    fn block_comment<'a>(&mut self, rest: &'a str, depth: usize) -> &'a str {
        let nested = self.is_psql().then(|| rest.find("/*")).flatten();
        match (nested, rest.find("*/")) {
            (Some(open), close) if close.is_none_or(|c| open < c) => {
                let (body, after) = rest.split_at(open + "/*".len());
                self.buf.push_str(body);
                self.block_comment(after, depth + 1)
            },
            (_, Some(close)) => {
                let (body, after) = rest.split_at(close + "*/".len());
                self.buf.push_str(body);
                if depth > 1 {
                    self.block_comment(after, depth - 1)
                } else {
                    self.state = Escaped::Normal(State::Plain);
                    after
                }
            },
            (_, None) => {
                self.buf.push_str(rest);
                self.state = Escaped::BlockComment(depth);
                ""
            },
        }
    }

    fn track_words(&mut self, mut ctx: State, mut rest: &str) -> State {
        loop {
            (ctx, rest) = self.resolve(ctx, rest);
            let Some((word, after)) = next_word(rest) else {
                return ctx;
            };
            ctx = self.keyword(word);
            rest = after;
        }
    }

    fn resolve<'a>(&mut self, ctx: State, rest: &'a str) -> (State, &'a str) {
        if matches!(ctx, State::Plain) {
            return (ctx, rest);
        }
        let token = rest.trim_start();
        if token.is_empty() {
            return (ctx, rest);
        }
        let (word, after) = token.split_at(
            token.find(|c: char| !is_word_char(c)).unwrap_or(token.len()),
        );
        match ctx {
            State::Plain => unreachable!("returned above"),
            State::AfterEnd if is_end_closer(word) => (State::Plain, after),
            State::AfterEnd => (State::Plain, rest),
            State::AfterIf => {
                if word
                    .starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                    && !eq_kw(word, "not")
                    && !eq_kw(word, "exists")
                {
                    self.depth += 1;
                }
                (State::Plain, rest)
            },
        }
    }

    fn keyword(&mut self, word: &str) -> State {
        let opener = eq_kw(word, "begin")
            || eq_kw(word, "case")
            || (self.depth > 0
                && ["loop", "while", "repeat"]
                    .iter()
                    .any(|kw| eq_kw(word, kw)));
        if opener {
            self.depth += 1;
        } else if eq_kw(word, "if") {
            if self.depth > 0 {
                return State::AfterIf;
            }
        } else if eq_kw(word, "end") {
            self.depth = self.depth.saturating_sub(1);
            return State::AfterEnd;
        } else if eq_kw(word, "commit") || eq_kw(word, "rollback") {
            self.depth = self.depth.saturating_sub(1);
        }
        State::Plain
    }
}

fn push_stmt(stmts: &mut Vec<String>, chunk: &str) {
    let stmt = chunk.trim();
    if !stmt.is_empty() {
        stmts.push(stmt.to_string());
    }
}

fn quote_state(close: char, escapes: bool) -> Escaped {
    match close {
        '\'' => Escaped::SingleQuote { escapes },
        '"' => Escaped::DoubleQuote,
        '`' => Escaped::Backtick,
        _ => Escaped::Bracket,
    }
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn eq_kw(word: &str, kw: &str) -> bool {
    word.eq_ignore_ascii_case(kw)
}

fn is_end_closer(word: &str) -> bool {
    ["if", "loop", "while", "repeat", "case"].iter().any(|kw| eq_kw(word, kw))
}

fn next_word(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start_matches(|c: char| !is_word_char(c));
    if s.is_empty() {
        return None;
    }
    Some(s.split_at(s.find(|c: char| !is_word_char(c)).unwrap_or(s.len())))
}

fn estring_prefix(buf: &str) -> bool {
    let mut chars = buf.chars().rev();
    matches!(chars.next(), Some('e' | 'E'))
        && !chars.next().is_some_and(is_word_char)
}

fn escaped(body: &str) -> bool {
    body.chars().rev().take_while(|&c| c == '\\').count() % 2 == 1
}

fn dollar_opener(s: &str) -> Option<(String, &str)> {
    let body = s.strip_prefix('$')?;
    let (tag, after) = body
        .split_at(body.find(|c: char| !is_word_char(c)).unwrap_or(body.len()));
    let after = after.strip_prefix('$')?;
    (!tag.starts_with(|c: char| c.is_ascii_digit()))
        .then(|| (format!("${tag}$"), after))
}
