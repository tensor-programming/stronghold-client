use vault::{BoxProvider, DBView, DBWriter, Id, Key, RecordHint};

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    cache::{send_until_success, CRequest},
    line_error, Cache,
};

#[derive(Clone)]
pub struct Client<P: BoxProvider> {
    id: Id,
    vault: Vault<P>,
    cache: Cache<Vec<u8>, Vec<u8>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Vault<P: BoxProvider> {
    ids: HashMap<Id, Key<P>>,
    dbs: HashMap<Key<P>, Option<DBView<P>>>,
}

#[derive(Serialize, Deserialize)]
pub struct Snapshot<P: BoxProvider> {
    pub ids: HashSet<Id>,
    pub keys: HashSet<Key<P>>,
    state: HashMap<Vec<u8>, Vec<u8>>,
}

impl<P: BoxProvider + Send + Sync + 'static> Client<P> {
    pub fn new(key: Key<P>, id: Id) -> Self {
        let cache = Cache::new();
        let vault = Vault::new(cache.clone(), &key, id);

        Self { id, vault, cache }
    }

    pub fn init_chain(key: Key<P>, id: Id) -> Self {
        let req = DBWriter::<P>::create_chain(&key, id);

        let cache = Cache::new();

        send_until_success(cache.clone(), CRequest::Write(req));

        let vault = Vault::new(cache.clone(), &key, id);

        Self { id, vault, cache }
    }

    pub fn add_new_chain(self, key: Key<P>, id: Id) -> Self {
        let req = DBWriter::<P>::create_chain(&key, id);

        let cache = Cache::new();

        send_until_success(cache.clone(), CRequest::Write(req));

        let vault = self.vault.add(cache.clone(), &key, id);

        Self { id, vault, cache }
    }

    pub fn create_record(self, key: Key<P>, payload: Vec<u8>) -> Id {
        let cache = self.cache.clone();
        let sid = self.id.clone();

        self.vault.take_view(self.cache.clone(), key, |view| {
            let (id, req) = view
                .writer(sid)
                .write(&payload, RecordHint::new(b"").expect(line_error!()))
                .expect(line_error!());

            req.into_iter().for_each(|req| {
                send_until_success(cache.clone(), CRequest::Write(req));
            });

            id
        })
    }

    pub fn list_all_records(self, key: Key<P>) {
        self.vault.take_view(self.cache, key, |view| {
            view.records()
                .for_each(|(id, hint)| println!("id: {:?}, hint: {:?}", id, hint))
        });
    }

    pub fn list_all_ids(self, key: Key<P>) {
        self.vault.take_view(self.cache, key, |view| {
            view.all().for_each(|id| println!("Id: {:?}", id))
        });
    }

    pub fn list_all_owned_keys(self, id: Id) {
        println!("{:?}", self.vault.ids.get(&id).expect(line_error!()));
    }
}

impl<P: BoxProvider> Vault<P> {
    pub fn new(cache: Cache<Vec<u8>, Vec<u8>>, key: &Key<P>, id: Id) -> Self {
        let mut ids: HashMap<Id, Key<P>> = HashMap::new();
        let mut dbs: HashMap<Key<P>, Option<DBView<P>>> = HashMap::new();

        let req = send_until_success(cache, CRequest::List).list();
        let view = DBView::load(key.clone(), req).expect(line_error!());

        ids.insert(id, key.clone());
        dbs.insert(key.clone(), Some(view));

        Self { ids, dbs }
    }

    pub fn add(mut self, cache: Cache<Vec<u8>, Vec<u8>>, key: &Key<P>, id: Id) -> Self {
        if self.dbs.get(&key).is_some() {
            self
        } else {
            let req = send_until_success(cache, CRequest::List).list();
            let view = DBView::load(key.clone(), req).expect(line_error!());

            let key = self.ids.get_mut(&id).expect(line_error!());

            self.dbs.insert(key.clone(), Some(view));

            self
        }
    }

    pub fn take_view<T>(self, cache: Cache<Vec<u8>, Vec<u8>>, key: Key<P>, mut f: impl FnMut(DBView<P>) -> T) -> T {
        let mut dbs = self.dbs;

        let mut _view = dbs.get_mut(&key).expect(line_error!());
        let view = _view.take().expect(line_error!());

        let retval = f(view);

        let req = send_until_success(cache.clone(), CRequest::List).list();
        *_view = Some(DBView::load(key.clone(), req).expect(line_error!()));

        retval
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

        let client = Client::init_chain(key.clone(), id);

        println!("{:?}", client.cache);

        client.create_record(key, b"data".to_vec());
    }
}
