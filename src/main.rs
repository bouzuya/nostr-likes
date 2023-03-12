use std::time::Duration;

use anyhow::ensure;
use bech32::{FromBase32, Variant};
use nostr_sdk::{
    prelude::{ToBech32, XOnlyPublicKey},
    Client, Filter, Keys, Kind, Options, RelayOptions,
};

#[derive(Debug, clap::Parser)]
struct Args {
    public_key: String,
    #[arg(long)]
    relay: Option<String>,
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

    println!("{:?}", events);

    Ok(())
}
