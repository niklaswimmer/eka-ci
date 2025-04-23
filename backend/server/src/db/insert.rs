use anyhow;
use sqlx;
use sqlx::SqlitePool;

use super::model::{build::DrvBuildMetadata, ForInsert};

pub async fn new_drv_build_metadata(
    metadata: ForInsert<DrvBuildMetadata>,
    pool: &SqlitePool,
) -> anyhow::Result<DrvBuildMetadata> {
    let metadata = metadata.0;
    let metadata = sqlx::query_as(
        r#"
INSERT INTO DrvBuildMetadata
    (derivation, git_repo, git_commit, build_command, build_attempt)
VALUES (
    ?1, ?2, ?3, ?4,
    IFNULL(
        (SELECT MAX(build_attempt) + 1
            FROM DrvBuildMetadata
            WHERE derivation = ?1),
        1
    )
)
RETURNING derivation, build_attempt, git_repo, git_commit, build_command
        "#,
    )
    .bind(&metadata.build.derivation)
    .bind(&metadata.git_repo)
    .bind(&metadata.git_commit)
    .bind(&metadata.build_command)
    .fetch_one(pool)
    .await?;

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use crate::db::model::{
        build::{DrvBuildCommand, DrvId},
        git::{GitCommit, GitRepo},
    };

    use super::*;

    #[sqlx::test(migrations = "./sql/migrations")]
    async fn insert_metadata_new_drv(pool: SqlitePool) -> anyhow::Result<()> {
        let metadata = DrvBuildMetadata::for_insert(
            DrvId::dummy(),
            GitRepo(gix_url::parse(
                "https://github.com/ekala-project/eka-ci".into(),
            )?),
            GitCommit(gix_hash::ObjectId::from_hex(
                b"ad7fb3f7660de7435baf14af66edef106dcffff9",
            )?),
            DrvBuildCommand::dummy(),
        );

        let inserted = new_drv_build_metadata(metadata.clone(), &pool).await?;

        // some sanity checks that the correct metadata object is returned
        assert_eq!(&inserted.build.derivation, &metadata.0.build.derivation);
        assert_eq!(&inserted.git_repo, &metadata.0.git_repo);
        assert_eq!(&inserted.git_commit, &metadata.0.git_commit);
        assert_eq!(&inserted.build_command, &metadata.0.build_command);

        // check that the build attempt was assigned correctly
        assert_eq!(inserted.build.build_attempt.get(), 1u32);

        Ok(())
    }

    #[sqlx::test(migrations = "./sql/migrations")]
    async fn insert_metadata_existing_drv(pool: SqlitePool) -> anyhow::Result<()> {
        sqlx::query(
            r#"
INSERT INTO DrvBuildMetadata (derivation, build_attempt, git_repo, git_commit, build_command)
VALUES (?, 1, 'https://github.com/ekala-project/corepkgs', '0000000000000000000000000000000000000000', ?)
            "#,
        )
        .bind(DrvId::dummy())
        .bind(DrvBuildCommand::dummy())
        .execute(&pool)
        .await?;

        let metadata = DrvBuildMetadata::for_insert(
            DrvId::dummy(),
            GitRepo(gix_url::parse(
                "https://github.com/ekala-project/corepkgs".into(),
            )?),
            GitCommit(gix_hash::ObjectId::from_hex(
                b"1111111111111111111111111111111111111111",
            )?),
            DrvBuildCommand::dummy(),
        );

        // should create a new entry with build attempt counter set to 2 because another entry
        // for the same derivation already exists
        let inserted = new_drv_build_metadata(metadata.clone(), &pool).await?;

        // some sanity checks that the correct metadata object is returned
        assert_eq!(&inserted.build.derivation, &metadata.0.build.derivation);
        assert_eq!(&inserted.git_repo, &metadata.0.git_repo);
        assert_eq!(&inserted.git_commit, &metadata.0.git_commit);
        assert_eq!(&inserted.build_command, &metadata.0.build_command);

        // check that the build attempt was assigned correctly
        assert_eq!(inserted.build.build_attempt.get(), 2u32);

        Ok(())
    }
}
