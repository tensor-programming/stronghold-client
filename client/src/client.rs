use vault::{BoxProvider, DBView, DBWriter, Id, Key, RecordHint};

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    cache::{send, CRequest, CResult},
    line_error,
};

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

        send(CRequest::Write(req));

        let req = send(CRequest::List).list();
        let view = DBView::<P>::load(key.clone(), req).expect(line_error!());

        let mut vaults = self.vaults;
        vaults.insert(key.clone(), Some(view));

        Self { vaults: vaults, ..self }
    }

    pub fn create_record(&mut self, key: Key<P>, payload: Vec<u8>) -> Option<Id> {
        let uid = self.id;

        self.take(key, |view| {
            let id = if let Some(v) = view {
                let (id, req) = v
                    .writer(uid)
                    .write(&payload, RecordHint::new(b"").expect(line_error!()))
                    .expect(line_error!());
                req.into_iter().for_each(|r| {
                    send(CRequest::Write(r));
                });
                Some(id)
            } else {
                None
            };

            id
        })
    }

    pub fn read_record(&mut self, key: Key<P>, id: Id) {
        self.take(key, |view| {
            if let Some(v) = view {
                let read = v.reader().prepare_read(id).expect("unable to read id");
                if let CResult::Read(read) = send(CRequest::Read(read)) {
                    let record = v.reader().read(read).expect(line_error!());
                    println!("Plain: {:?}", String::from_utf8(record).unwrap());
                }
            }
        });
    }

    pub fn preform_gc(&mut self, key: Key<P>) {
        let uid = self.id;
        self.take(key, |view| {
            if let Some(v) = view {
                let (to_write, to_delete) = v.writer(uid).gc().expect(line_error!());
                to_write.into_iter().for_each(|r| {
                    send(CRequest::Write(r));
                });
                to_delete.into_iter().for_each(|r| {
                    send(CRequest::Delete(r));
                });
            };
        })
    }

    pub fn revoke_record_by_id(&mut self, id: Id, key: Key<P>) {
        let uid = self.id;
        self.take(key, |view| {
            if let Some(v) = view {
                let (to_write, to_delete) = v.writer(uid).revoke(id).expect(line_error!());

                send(CRequest::Write(to_write));
                send(CRequest::Delete(to_delete));
            };
        })
    }

    pub fn list_valid_ids_for_vault(&mut self, key: Key<P>) {
        self.take(key, |view| {
            if let Some(v) = view {
                v.records()
                    .for_each(|(id, hint)| println!("Id: {:?}, Hint: {:?}", id, hint))
            };
        });
    }

    pub fn take<T>(&mut self, key: Key<P>, f: impl FnOnce(Option<DBView<P>>) -> T) -> T {
        let vault = self.vaults.remove(&key).expect(line_error!());

        let ret = f(vault);

        let req = send(CRequest::List).list();
        self.vaults
            .insert(key.clone(), Some(DBView::load(key, req).expect(line_error!())));

        ret
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
        client.list_valid_ids_for_vault(key.clone());
        client.list_valid_ids_for_vault(key_2.clone());
        client.read_record(key_2.clone(), tx_id_2.unwrap());
        client.read_record(key, tx_id.unwrap());

        client.revoke_record_by_id(tx_id_2.unwrap(), key_2.clone());

        client.preform_gc(key_2);
    }
}
