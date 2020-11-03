use std::{collections::HashMap, fmt::Debug};

use vault::{
    BoxProvider, DBView, DBWriter, DeleteRequest, Id, Key, ListResult, ReadRequest, ReadResult, RecordHint,
    WriteRequest,
};

use crate::line_error;

pub trait Bucket {
    fn send(&mut self, req: CRequest) -> CResult;
}

pub trait Vault<P: BoxProvider + Send + Sync + 'static> {
    fn create_record(&mut self, uid: Id, key: Key<P>, payload: Vec<u8>) -> Option<Id>;
    fn add_vault(&mut self, key: &Key<P>, uid: Id);
    fn read_record(&mut self, uid: Id, key: Key<P>);
    fn garbage_collect(&mut self, uid: Id, key: Key<P>);
    fn revoke_record(&mut self, uid: Id, tx_id: Id, key: Key<P>);
    fn list_all_valid_by_key(&mut self, key: Key<P>);
}

pub struct Vaults<P: BoxProvider + Send + Sync + 'static> {
    vaults: HashMap<Key<P>, Option<DBView<P>>>,
    cache: Cache,
}

#[derive(Clone, Debug)]
pub struct Value<T>(T);

#[derive(Clone, Debug)]
pub struct Cache {
    table: HashMap<Vec<u8>, Value<Vec<u8>>>,
}

#[derive(Clone)]
pub enum CRequest {
    List,
    Write(WriteRequest),
    Delete(DeleteRequest),
    Read(ReadRequest),
}

#[derive(Clone)]
pub enum CResult {
    List(ListResult),
    Write,
    Delete,
    Read(ReadResult),
}

impl Cache {
    pub fn new() -> Self {
        Cache { table: HashMap::new() }
    }

    fn add_data(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.table.insert(key, Value::new(value));
    }

    fn read_data(&self, key: Vec<u8>) -> Value<Vec<u8>> {
        self.table.get(&key).expect(line_error!()).clone()
    }

    pub fn offload_data(self) -> HashMap<Vec<u8>, Vec<u8>> {
        let mut ret: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

        self.table.into_iter().for_each(|(k, v)| {
            ret.insert(k, v.0);
        });

        ret
    }

    pub fn upload_data(mut self, map: HashMap<Vec<u8>, Vec<u8>>) {
        map.into_iter().for_each(|(k, v)| {
            self.table.insert(k, Value::new(v));
        });
    }
}

impl Bucket for Cache {
    fn send(&mut self, req: CRequest) -> CResult {
        let result = match req {
            CRequest::List => {
                let entries = self.table.keys().cloned().collect();
                CResult::List(ListResult::new(entries))
            }
            CRequest::Write(write) => {
                self.add_data(write.id().to_vec(), write.data().to_vec());
                CResult::Write
            }
            CRequest::Delete(del) => {
                self.table.retain(|id, _| *id != del.id());
                CResult::Delete
            }
            CRequest::Read(read) => {
                let state = self.read_data(read.id().to_vec());
                CResult::Read(ReadResult::new(read.into(), state.0))
            }
        };
        result
    }
}

impl<P: BoxProvider + Send + Sync + 'static> Vaults<P> {
    pub fn new() -> Self {
        let cache = Cache::new();
        let vaults = HashMap::new();

        Self { cache, vaults }
    }

    pub fn get_view(&mut self, key: &Key<P>) -> Option<DBView<P>> {
        self.vaults.remove(key).expect(line_error!())
    }

    pub fn reset_view(&mut self, key: Key<P>) {
        let req = self.cache.send(CRequest::List).list();
        self.vaults
            .insert(key.clone(), Some(DBView::load(key, req).expect(line_error!())));
    }
}

impl<P: BoxProvider + Send + Sync + 'static> Vault<P> for Vaults<P> {
    fn create_record(&mut self, uid: Id, key: Key<P>, payload: Vec<u8>) -> Option<Id> {
        let view = self.get_view(&key);

        let id = if let Some(v) = view {
            let (id, req) = v
                .writer(uid)
                .write(&payload, RecordHint::new(b"").expect(line_error!()))
                .expect(line_error!());
            req.into_iter().for_each(|r| {
                self.cache.send(CRequest::Write(r));
            });
            Some(id)
        } else {
            None
        };

        self.reset_view(key);

        id
    }

    fn add_vault(&mut self, key: &Key<P>, uid: Id) {
        let req = DBWriter::<P>::create_chain(&key, uid);

        self.cache.send(CRequest::Write(req));

        self.reset_view(key.clone());
    }

    fn read_record(&mut self, uid: Id, key: Key<P>) {
        let view = self.get_view(&key);
        if let Some(v) = view {
            let read = v.reader().prepare_read(uid).expect("unable to read id");
            if let CResult::Read(read) = self.cache.send(CRequest::Read(read)) {
                let record = v.reader().read(read).expect(line_error!());
                println!("Plain: {:?}", String::from_utf8(record).unwrap());
            }
        }

        self.reset_view(key);
    }

    fn garbage_collect(&mut self, uid: Id, key: Key<P>) {
        let view = self.get_view(&key);

        if let Some(v) = view {
            let (write, delete) = v.writer(uid).gc().expect(line_error!());
            write.into_iter().for_each(|r| {
                self.cache.send(CRequest::Write(r));
            });

            delete.into_iter().for_each(|r| {
                self.cache.send(CRequest::Delete(r));
            });
        }
        self.reset_view(key);
    }

    fn revoke_record(&mut self, uid: Id, tx_id: Id, key: Key<P>) {
        let view = self.get_view(&key);

        if let Some(v) = view {
            let (to_write, to_delete) = v.writer(uid).revoke(tx_id).expect(line_error!());

            self.cache.send(CRequest::Write(to_write));
            self.cache.send(CRequest::Delete(to_delete));
        };

        self.reset_view(key);
    }

    fn list_all_valid_by_key(&mut self, key: Key<P>) {
        let view = self.get_view(&key);

        if let Some(v) = view {
            v.records()
                .for_each(|(id, hint)| println!("Id: {:?}, Hint: {:?}", id, hint))
        }

        self.reset_view(key);
    }
}

impl<T> Value<T> {
    pub fn new(val: T) -> Self {
        Self(val)
    }
}

impl CResult {
    pub fn list(self) -> ListResult {
        match self {
            CResult::List(list) => list,
            _ => panic!(line_error!()),
        }
    }
}
