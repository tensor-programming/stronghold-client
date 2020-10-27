use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::RwLock};

use vault::{DeleteRequest, ListResult, ReadRequest, ReadResult, WriteRequest};

use crate::line_error;

use once_cell::sync::Lazy;

static CACHE: Lazy<RwLock<Cache>> = Lazy::new(|| RwLock::new(Cache::new()));

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

    pub fn add_data(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.table.insert(key, Value::new(value));
    }

    pub fn read_data(&self, key: Vec<u8>) -> Value<Vec<u8>> {
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

impl<T> Value<T> {
    pub fn new(val: T) -> Self {
        Self(val)
    }
}

pub fn send(req: CRequest) -> CResult {
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

            CResult::Read(ReadResult::new(read.into(), state.0))
        }
    };

    result
}

impl CResult {
    pub fn list(self) -> ListResult {
        match self {
            CResult::List(list) => list,
            _ => panic!(line_error!()),
        }
    }
}
