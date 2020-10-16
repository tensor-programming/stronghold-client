use std::{collections::HashMap, fmt::Debug, hash::Hash};

use vault::{DeleteRequest, ListResult, ReadRequest, ReadResult, WriteRequest};

use std::{thread, time::Duration};

use crate::line_error;

#[derive(Clone, Debug)]
pub struct Value<T>(T);

#[derive(Clone, Debug)]
pub struct Cache<K, V>
where
    K: Hash + Eq,
    V: Clone + Debug,
{
    table: HashMap<K, Value<V>>,
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

impl<T> Value<T> {
    pub fn new(val: T) -> Self {
        Self(val)
    }
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
        self.table.insert(key, Value::new(value));
    }

    pub fn read_data(self, key: K) -> Value<V> {
        self.table.get(&key).expect(line_error!()).clone()
    }

    pub fn offload_data(self) -> HashMap<K, V> {
        let mut ret: HashMap<K, V> = HashMap::new();

        self.table.into_iter().for_each(|(k, v)| {
            ret.insert(k, v.0);
        });

        ret
    }

    pub fn upload_data(mut self, map: HashMap<K, V>) {
        map.into_iter().for_each(|(k, v)| {
            self.table.insert(k, Value::new(v));
        });
    }
}

pub fn send(mut cache: Cache<Vec<u8>, Vec<u8>>, req: CRequest) -> Option<CResult> {
    let result = match req {
        CRequest::List => {
            let entries = cache.table.keys().cloned().collect();

            CResult::List(ListResult::new(entries))
        }

        CRequest::Write(write) => {
            cache
                .table
                .insert(write.id().to_vec(), Value::new(write.data().to_vec()));

            CResult::Write
        }
        CRequest::Delete(del) => {
            cache.table.retain(|id, _| *id != del.id());

            CResult::Delete
        }
        CRequest::Read(read) => {
            let state = cache.table.get(read.id()).cloned().expect(line_error!());

            CResult::Read(ReadResult::new(read.into(), state.0))
        }
    };

    Some(result)
}

pub fn send_until_success(cache: Cache<Vec<u8>, Vec<u8>>, req: CRequest) -> CResult {
    loop {
        match send(cache.clone(), req.clone()) {
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
