use super::*;

use sea_orm::{Database, DeriveEntityModel, Schema};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "foos")]
struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    #[sea_orm(column_type = "Text")]
    name: String,
}

#[tokio::test]
async fn no_panic() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let schema = Schema::new(DbBackend::Sqlite);
    let stmt = schema.create_table_from_entity(Foo);
    let result = db.execute(db.get_database_backend().build(&stmt)).await;
}
