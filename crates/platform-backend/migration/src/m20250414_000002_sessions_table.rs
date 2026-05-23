use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Sessions::Table)
                    .if_not_exists()
                    .col(string(Sessions::Id)) // UUID as primary key
                    .col(integer(Sessions::UserId))
                    .col(string(Sessions::ClientId)) // UUID
                    .col(json(Sessions::Data))
                    .col(
                        timestamp_with_time_zone(Sessions::ExpiresAt)
                            .not_null(),
                    )
                    .col(
                        timestamp_with_time_zone(Sessions::CreatedAt)
                            .default(Expr::current_timestamp()),
                    )
                    .primary_key(
                        Index::create()
                            .col(Sessions::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on user_id for fast session lookup
        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_user_id")
                    .table(Sessions::Table)
                    .col(Sessions::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Sessions {
    Table,
    Id,
    UserId,
    ClientId,
    Data,
    ExpiresAt,
    CreatedAt,
}
