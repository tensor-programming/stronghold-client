use std::{collections::HashMap, fmt::Debug, hash::Hash};

use vault::{DeleteRequest, ListResult, ReadRequest, ReadResult, WriteRequest};

use std::{thread, time::Duration};

use crate::line_error;

use once_cell::sync::Lazy;

use std::sync::RwLock;

static CACHE: Lazy<RwLock<Cache<Vec<u8>, Vec<u8>>>> = Lazy::new(|| RwLock::new(Cache::new()));

#[derive(Clone, Debug)]
pub struct Cache<K, V>
where
    K: Hash + Eq,
    V: Clone + Debug,
{
    table: HashMap<K, V>,
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

impl<K, V> Cache<K, V>
where
    K: Hash + Eq,
    V: Clone + Debug,
{
    pub fn new() -> Self {
        Cache { table: HashMap::new() }
    }

    pub fn add_data(&mut self, key: K, value: V) {
        self.table.insert(key, value);
    }

    pub fn read_data(&self, key: K) -> V {
        self.table.get(&key).expect(line_error!()).clone()
    }

    pub fn offload_data(self) -> HashMap<K, V> {
        let mut ret: HashMap<K, V> = HashMap::new();

        self.table.into_iter().for_each(|(k, v)| {
            ret.insert(k, v);
        });

        ret
    }

    pub fn upload_data(mut self, map: HashMap<K, V>) {
        map.into_iter().for_each(|(k, v)| {
            self.table.insert(k, v);
        });
    }
}

pub fn send(req: CRequest) -> Option<CResult> {
    let result = match req {
        CRequest::List => {
            let entries = CACHE.read().expect(line_error!()).table.keys().cloned().collect();

            CResult::List(ListResult::new(entries))
        }

        CRequest::Write(write) => {
            CACHE
                .write()
                .expect(line_error!())
                .add_data(write.id().to_vec(), write.data().to_vec());

            CResult::Write
        }
        CRequest::Delete(del) => {
            CACHE
                .write()
                .expect(line_error!())
                .table
                .retain(|id, _| *id != del.id());

            CResult::Delete
        }
        CRequest::Read(read) => {
            let state = CACHE.read().expect(line_error!()).read_data(read.id().to_vec());

            CResult::Read(ReadResult::new(read.into(), state))
        }
    };

    Some(result)
}

pub fn send_until_success(req: CRequest) -> CResult {
    loop {
        match send(req.clone()) {
            Some(result) => {
                break result;
            }
            None => thread::sleep(Duration::from_millis(50)),
        }
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
