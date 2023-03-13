use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
    time::Duration,
};

use anyhow::ensure;
use bech32::{FromBase32, Variant};
use nostr_sdk::{
    prelude::{ToBech32, XOnlyPublicKey},
    Client, Event, EventId, Filter, Keys, Kind, Options, RelayOptions, Tag,
};
use xdg::BaseDirectories;

#[derive(Debug, clap::Parser)]
struct Args {
    public_key: String,
    #[arg(long)]
    relay: Option<String>,
}

fn cache_dir() -> anyhow::Result<PathBuf> {
    let prefix = "net.bouzuya.lab.nostr-likes";
    Ok(match env::var_os("NOSTR_LIKES_CACHE_DIR") {
        Some(cache_dir) => PathBuf::from(cache_dir),
        None => BaseDirectories::with_prefix(prefix)?.get_cache_home(),
    })
}

fn load() -> anyhow::Result<HashMap<EventId, Event>> {
    let path = cache_dir()?.join("events.json");
    if path.exists() {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let metadata_cache = serde_json::from_reader(reader)?;
        Ok(metadata_cache)
    } else {
        Ok(Default::default())
    }
}

fn store(cache: &HashMap<EventId, Event>) -> anyhow::Result<()> {
    let path = cache_dir()?.join("events.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string(cache)?;
    fs::write(path, content)?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = <Args as clap::Parser>::parse();

    let (hrp_lower, data, variant) = bech32::decode(&args.public_key)?;
    ensure!(hrp_lower == "npub", "only npub... is allowed");
    ensure!(variant == Variant::Bech32, "only bech32-(not-m) is allowed");
    let base32 = Vec::<u8>::from_base32(&data)?;
    // let base32_in_hex = base32
    //     .into_iter()
    //     .map(|b| format!("{:x}", b))
    //     .collect::<String>();

    // TODO: remove nostr-sdk
    let mut cached = load()?;

    let keys = Keys::generate();
    let client = Client::new_with_opts(&keys, Options::default().wait_for_send(true));
    let relay = args
        .relay
        .unwrap_or_else(|| "wss://relay.damus.io".to_owned());
    println!("{}", relay);
    client
        .add_relay_with_opts(relay, None, RelayOptions::new(true, false))
        .await?;
    client.connect().await;
    let public_key = XOnlyPublicKey::from_slice(&base32)?;
    println!("{}", public_key.to_bech32()?);
    let filter = Filter::new()
        .kind(Kind::Reaction)
        .author(public_key)
        .limit(10);
    let events = client
        .get_events_of(vec![filter], Some(Duration::from_secs(10)))
        .await?;

    for event in events {
        if let Some(event_id) = event.tags.iter().find_map(|tag| {
            if let Tag::Event(id, _, _) = tag {
                Some(id)
            } else {
                None
            }
        }) {
            match cached.get(event_id) {
                Some(event) => println!("{:?}", event),
                None => {
                    let filter = Filter::new().id(event_id.to_string()).limit(1);
                    let events = client
                        .get_events_of(vec![filter], Some(Duration::from_secs(10)))
                        .await?;
                    if !events.is_empty() {
                        let event = events[0].clone();
                        println!("{:?}", event);
                        cached.insert(event.id, event);
                    }
                }
            }
        }
    }

    store(&cached)?;

    Ok(())
}
