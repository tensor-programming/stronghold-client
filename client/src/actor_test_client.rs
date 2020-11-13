use riker::actors::*;

use vault::{BoxProvider, Id, Key};

use crate::provider::Provider;

use crate::data::{Blob, Bucket};

#[derive(Debug, Clone)]
pub struct Returns(Id);

#[derive(Debug, Clone)]
pub enum CMsg<P: BoxProvider> {
    AddVault(Key<P>),
    ListRecords(Key<P>),
    ReadRecord(Key<P>),
    RevokeRecord((Key<P>, Id)),
    CreateRecord((Key<P>, Vec<u8>)),
    GarbageCollect(Key<P>),
    Returns(Option<Id>),
}

#[derive(Debug, Clone)]
pub enum BMsg<P: BoxProvider> {
    AddVault((Key<P>, Id)),
    ListRecords(Key<P>),
    ReadRecord((Key<P>, Id)),
    RevokeRecord((Key<P>, Id, Id)),
    CreateRecord((Key<P>, Id, Vec<u8>)),
    GarbageCollect((Id, Key<P>)),
}

struct Client {
    id: Id,
    txids: Vec<Id>,
}

impl ActorFactoryArgs<Id> for Client {
    fn create_args(id: Id) -> Self {
        Client { id, txids: vec![] }
    }
}

impl Actor for Client {
    type Msg = CMsg<Provider>;

    fn recv(&mut self, ctx: &Context<CMsg<Provider>>, msg: CMsg<Provider>, sender: Sender) {
        self.receive(ctx, msg, sender);
    }
}

impl Receive<CMsg<Provider>> for Client {
    type Msg = CMsg<Provider>;

    fn receive(&mut self, ctx: &Context<Self::Msg>, msg: CMsg<Provider>, _sender: Sender) {
        match msg {
            CMsg::AddVault(key) => {
                let blob_actor = ctx.select("/user/blob/").unwrap();
                blob_actor.try_tell(BMsg::AddVault((key, self.id)), None);
            }
            CMsg::ListRecords(key) => {
                let blob_actor = ctx.select("/user/blob/").unwrap();
                blob_actor.try_tell(BMsg::ListRecords(key), None);
            }
            CMsg::CreateRecord((key, payload)) => {
                let blob_actor = ctx.select("/user/blob/").unwrap();
                blob_actor.try_tell(BMsg::CreateRecord((key, self.id, payload)), None);
            }
            CMsg::ReadRecord(key) => {
                let blob_actor = ctx.select("/user/blob/").unwrap();
                blob_actor.try_tell(BMsg::ReadRecord((key, self.id)), None);
            }
            CMsg::RevokeRecord((key, id)) => {
                let blob_actor = ctx.select("/user/blob/").unwrap();
                blob_actor.try_tell(BMsg::RevokeRecord((key, self.id, id)), None);
            }
            CMsg::GarbageCollect(key) => {
                let blob_actor = ctx.select("/user/blob/").unwrap();
                blob_actor.try_tell(BMsg::GarbageCollect((self.id, key)), None);
            }
            CMsg::Returns(txid) => {
                println!("Created and Got {:?}", txid.unwrap());
                self.txids.push(txid.unwrap());
            }
        }
    }
}

impl ActorFactory for Blob<Provider> {
    fn create() -> Self {
        Blob::new()
    }
}

impl Actor for Blob<Provider> {
    type Msg = BMsg<Provider>;

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
    }
}

impl Receive<BMsg<Provider>> for Blob<Provider> {
    type Msg = BMsg<Provider>;

    fn receive(&mut self, ctx: &Context<Self::Msg>, msg: BMsg<Provider>, _sender: Sender) {
        match msg {
            BMsg::AddVault((key, id)) => {
                self.add_vault(&key, id);
            }
            BMsg::ListRecords(key) => {
                self.list_all_valid_by_key(key);
            }
            BMsg::CreateRecord((key, id, payload)) => {
                let tx_id = self.create_record(id, key, payload);
                let client = ctx.select("/user/client/").unwrap();

                client.try_tell(CMsg::<Provider>::Returns(tx_id), None);
            }
            BMsg::ReadRecord((key, tx_id)) => {
                self.read_record(tx_id, key);
            }
            BMsg::RevokeRecord((key, uid, tx_id)) => {
                self.revoke_record(uid, tx_id, key);
            }
            BMsg::GarbageCollect((id, key)) => {
                self.garbage_collect(id, key);
            }
            _ => {}
        }
    }
}

#[test]
fn test_actor_system() {
    let key = Key::<Provider>::random().expect("Couldn't create key");
    let sys = ActorSystem::new().unwrap();

    let client = sys
        .actor_of_args::<Client, _>("client", Id::random::<Provider>().unwrap())
        .unwrap();

    sys.actor_of::<Blob<Provider>>("blob").unwrap();

    client.tell(CMsg::AddVault(key.clone()), None);
    client.tell(CMsg::CreateRecord((key.clone(), b"Some data".to_vec())), None);

    std::thread::sleep(std::time::Duration::from_millis(500));
    client.tell(CMsg::ListRecords(key), None);

    std::thread::sleep(std::time::Duration::from_millis(500));
}
