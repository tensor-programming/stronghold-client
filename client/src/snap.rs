use snapshot::{decrypt_snapshot, encrypt_snapshot, snapshot_dir};

use std::{fs::OpenOptions, path::PathBuf};

use crate::{
    client::{Client, Snapshot},
    provider::Provider,
};

fn get_snapshot_path() -> PathBuf {
    let path = snapshot_dir().expect("Unable to get the snapshot path");

    path.join("backup.snapshot")
}

pub fn deserialize_from_snapshot(snapshot: &PathBuf, pass: &str) -> Client<Provider> {
    let mut buffer = Vec::new();

    let mut file = OpenOptions::new()
        .read(true)
        .open(snapshot)
        .expect("Unable to access the snapshot. Make sure it exists.");

    decrypt_snapshot(&mut file, &mut buffer, pass.as_bytes());

    let snapshot: Snapshot<Provider> = bincode::deserialize(&buffer[..]).expect("Unable to deserialize data");

    Client::<Provider>::new_from_snapshot(snapshot)
}

pub fn serialize_to_snapshot(snapshot: &PathBuf, pass: &str, mut client: Client<Provider>) {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(snapshot)
        .expect("Unable to access snapshot. Make sure that it exists.");

    file.set_len(0).expect("unable to clear the contents of the file file");

    let snap: Snapshot<Provider> = Snapshot::new(&mut client);

    let data: Vec<u8> = bincode::serialize(&snap).expect("Couldn't serialize the client data");
    encrypt_snapshot(data, &mut file, pass.as_bytes()).expect("Couldn't write to the snapshot");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Blob;
    use crate::line_error;
    use crate::provider::Provider;
    use vault::{Id, Key};

    #[test]
    fn test_snapshots() {
        let id = Id::random::<Provider>().expect(line_error!());
        let key = Key::<Provider>::random().expect(line_error!());

        let blobs = Blob::new();

        let mut client = Client::new(id, blobs);
        client.add_vault(&key);

        let tx_id = client.create_record(key.clone(), b"data".to_vec());

        let key_2 = Key::<Provider>::random().expect(line_error!());

        let key_3 = Key::<Provider>::random().expect(line_error!());

        client.add_vault(&key_2);

        let tx_id_2 = client.create_record(key_2.clone(), b"more_data".to_vec());
        client.list_valid_ids_for_vault(key.clone());
        client.list_valid_ids_for_vault(key_2.clone());
        client.read_record(key_2.clone(), tx_id_2.unwrap());
        client.read_record(key.clone(), tx_id.unwrap());

        client.add_vault(&key_3.clone());
        let tx_id_3 = client.create_record(key_3.clone(), b"3rd vault data".to_vec());
        client.read_record(key_3.clone(), tx_id_3.unwrap());
        client.list_valid_ids_for_vault(key_3.clone());

        let snapshot_path = PathBuf::from("./test.snapshot");

        serialize_to_snapshot(&snapshot_path, "password", client);

        let mut client = deserialize_from_snapshot(&snapshot_path, "password");

        client.preform_gc(key.clone());
        client.preform_gc(key_2.clone());
        client.preform_gc(key_3.clone());

        let tx_id_4 = client.create_record(key_3.clone(), b"Another Piece of data in the 3rd vault".to_vec());

        client.list_valid_ids_for_vault(key_2.clone());
        client.list_valid_ids_for_vault(key.clone());
        client.list_valid_ids_for_vault(key_3.clone());

        client.read_record(key.clone(), tx_id.unwrap());
        client.read_record(key_3.clone(), tx_id_3.unwrap());
        client.read_record(key_2.clone(), tx_id_2.unwrap());
        client.read_record(key_3, tx_id_4.unwrap());
    }
}
