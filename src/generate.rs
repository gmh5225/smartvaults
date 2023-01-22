
use clap::{Parser, Error};
use nostr_rust::bech32::{to_bech32, ToBech32Kind};
use std::str::FromStr;
use bdk::keys::{
    bip39::{WordCount},
    GeneratableKey, GeneratedKey,
};
use bdk::wallet::Wallet;
use bitcoin::util::bip32;
use bdk::miniscript;
use secp256k1::Secp256k1;
use bip39::{Mnemonic, Language};

fn generate(passphrase: &String) {

    let mnemonic: GeneratedKey<_, miniscript::Segwitv0> =
        Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();

    // Convert mnemonic to string
    let mnemonic_words = mnemonic.to_string();
    println!("Mnemonic : {:?} ", &mnemonic_words);

    // Parse a mnemonic
    let mnemonic = Mnemonic::parse(&mnemonic_words).unwrap();

    let seed = mnemonic.to_seed_normalized(passphrase);
    println!("seed: {:?}", seed);

    let path = bip32::DerivationPath::from_str("m/44'/0'/0'/0").unwrap();

    let key = (mnemonic, path);
    let (desc, _keys, _networks) = bdk::descriptor!(wpkh(key)).unwrap();
    println!("Bitcoin Output Descriptor: {}", desc.to_string());

    let db = bdk::database::memory::MemoryDatabase::new();
    let wallet = Wallet::new(desc, None, bitcoin::Network::Bitcoin, db);
    let address = wallet
        .unwrap()
        .get_address(bdk::wallet::AddressIndex::New)
        .unwrap();
    println!("First Address : {} ", address.to_string());

    let secp = Secp256k1::new();

    // mnemonic creates 64-bytes, but we only use the first 32
    let secret_key = secp256k1::SecretKey::from_slice(&seed[0..32]).unwrap();
    let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

    let secret_key_str = secret_key.display_secret().to_string();

    println!("Nostr Secret Key (HEX): {:?} ", secret_key_str);
    println!("Nostr Public Key (HEX): {:?} ", public_key.to_string());

    let bech32_pub = to_bech32(ToBech32Kind::PublicKey, &public_key.to_string());
    let bech32_prv = to_bech32(ToBech32Kind::SecretKey, &secret_key_str);

    println!("Nostr Public Key (bech32): {:?} ", bech32_pub.unwrap());
    println!("Nostr Secret Key (bech32): {:?} ", bech32_prv.unwrap());
}

/// The `generate` command
#[derive(Debug, Clone, Parser)]
#[command(name = "generate", about = "Generate a random account to work with Nostr and Bitcoin")]
pub struct GenerateCmd {
    /// The number of random accounts to generate
    #[arg(short, long, default_value_t = 1)]
    count: u8,

    #[arg(short, long, default_value = "")]
    passphrase: String,
}

impl GenerateCmd {
    pub fn run(&self) -> Result<(), Error> {
        for i in 0..self.count {
            println!("\nGenerating account {} of {}", i+1, self.count);
            generate(&self.passphrase);
            println!();
        }

        Ok(())
    }
}
