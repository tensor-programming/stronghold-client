use vault::{BoxProvider, DBView, DBWriter, Id, Key, RecordHint};

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    cache::{send_until_success, CRequest, CResult},
    line_error,
};
#[derive(Debug)]
pub struct Client<P: BoxProvider> {
    id: Id,
    vaults: HashMap<Key<P>, Option<DBView<P>>>,
}

#[derive(Serialize, Deserialize)]
pub struct Snapshot<P: BoxProvider> {
    pub ids: HashSet<Id>,
    pub keys: HashSet<Key<P>>,
    state: HashMap<Vec<u8>, Vec<u8>>,
}

impl<P: BoxProvider + Send + Sync + 'static> Client<P> {
    pub fn new(id: Id) -> Self {
        let vaults = HashMap::<Key<P>, Option<DBView<P>>>::new();

        Self { id, vaults }
    }

    pub fn add_vault(self, key: &Key<P>) -> Self {
        let req = DBWriter::<P>::create_chain(&key, self.id);

        send_until_success(CRequest::Write(req));

        let req = send_until_success(CRequest::List).list();
        let view = DBView::<P>::load(key.clone(), req).expect(line_error!());

        let mut vaults = self.vaults;
        vaults.insert(key.clone(), Some(view));

        Self { vaults: vaults, ..self }
    }

    pub fn create_record(&mut self, key: Key<P>, payload: Vec<u8>) -> Option<Id> {
        let vault = self.vaults.remove(&key).expect(line_error!());

        let id = if let Some(v) = vault {
            let (id, req) = v
                .writer(self.id)
                .write(&payload, RecordHint::new(b"").expect(line_error!()))
                .expect(line_error!());

            req.into_iter().for_each(|r| {
                send_until_success(CRequest::Write(r));
            });

            Some(id)
        } else {
            None
        };

        let req = send_until_success(CRequest::List).list();

        self.vaults
            .insert(key.clone(), Some(DBView::load(key, req).expect(line_error!())));

        id
    }

    pub fn read_record(&mut self, key: Key<P>, id: Id) {
        let vault = self.vaults.remove(&key).expect(line_error!());

        if let Some(v) = vault {
            let read = v.reader().prepare_read(id).expect("unable to read id");

            if let CResult::Read(read) = send_until_success(CRequest::Read(read)) {
                let record = v.reader().read(read).expect(line_error!());

                println!("Plain: {:?}", String::from_utf8(record).unwrap());
            }
        }

        let req = send_until_success(CRequest::List).list();

        self.vaults
            .insert(key.clone(), Some(DBView::load(key, req).expect(line_error!())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Provider;

    #[test]
    fn test_stuff() {
        let id = Id::random::<Provider>().expect(line_error!());
        let key = Key::<Provider>::random().expect(line_error!());

        let client = Client::new(id);
        let mut client = client.add_vault(&key);

        let tx_id = client.create_record(key.clone(), b"data".to_vec());

        let key_2 = Key::<Provider>::random().expect(line_error!());

        let mut client = client.add_vault(&key_2);

        let tx_id_2 = client.create_record(key_2.clone(), b"more_data".to_vec());
        client.read_record(key_2, tx_id_2.unwrap());
        client.read_record(key, tx_id.unwrap());
    }
}
