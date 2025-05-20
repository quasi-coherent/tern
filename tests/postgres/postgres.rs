use self::runner::TestRunner;

pub mod migrations;
pub mod runner;

#[tokio::test]
async fn pgtest_apply_all() {
    let mut test = TestRunner::new().await;
    let res = test.apply(None).await;
    assert!(matches!(res, Ok(n) if n == 4));
    assert_eq!(test.check().get_applied().await.len(), 4);
    test.drop_history().await.ok();
}

// #[tokio::test]
// async fn pgtest_apply() {
//     let mut test = TestRunner::new().await;
//     let res = test.apply(Some(3)).await;
//     assert!(matches!(res, Ok(n) if n == 3));
//     assert_eq!(test.check().get_applied().await.len(), 3);
//     let res = test.apply(None).await;
//     assert!(res.is_ok());
//     assert_eq!(test.check().get_applied().await.len(), 4);
//     test.drop_history().await.ok();
// }

// #[tokio::test]
// async fn pgtest_soft_apply() {
//     let mut test = TestRunner::new().await;
//     let res = test.soft_apply(Some(2)).await;
//     assert!(matches!(res, Ok(n) if n == 2));
//     assert_eq!(test.check().get_applied().await.len(), 2);
//     assert!(!test.check().users_table_exists().await);
//     test.drop_history().await.ok();
// }
