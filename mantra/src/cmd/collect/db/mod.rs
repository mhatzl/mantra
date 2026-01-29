use crate::db::{MantraConnection, MantraDb, MantraTransaction};

pub(crate) struct CollectTransaction<'db> {
    transaction: MantraTransaction<'db>,
}

impl<'db> CollectTransaction<'db> {
    pub(crate) async fn new(db: &'db MantraDb) -> Result<Self, anyhow::Error> {
        Ok(Self {
            transaction: db.start_transaction().await?,
        })
    }

    pub(crate) fn connection(&mut self) -> &mut MantraConnection {
        self.transaction.as_mut()
    }

    pub(crate) async fn commit(self) -> Result<(), anyhow::Error> {
        Ok(self.transaction.commit().await?)
    }
}
