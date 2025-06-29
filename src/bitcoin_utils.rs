use anyhow::{anyhow, Result};
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use bitcoin::{Address, Network, PublicKey as BitcoinPublicKey};
use num_bigint::BigUint;
use num_traits::Num;

pub fn parse_hex_key(hex_str: &str) -> Result<BigUint> {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    BigUint::from_str_radix(hex_str, 16).map_err(|e| anyhow!("Invalid hex key: {}", e))
}

pub fn private_key_to_addresses(private_key: &BigUint) -> Result<Vec<String>> {
    let secp = Secp256k1::new();

    // Convert BigUint to 32-byte array
    let key_bytes = private_key_to_bytes(private_key)?;

    // Create secp256k1 secret key
    let secret_key =
        SecretKey::from_slice(&key_bytes).map_err(|e| anyhow!("Invalid private key: {}", e))?;

    // Generate public key
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    let mut addresses = Vec::new();

    // Generate compressed address
    let compressed_pubkey = BitcoinPublicKey::new(public_key);
    let compressed_addr = Address::p2pkh(&compressed_pubkey, Network::Bitcoin);
    addresses.push(compressed_addr.to_string());

    // Generate uncompressed address
    let uncompressed_pubkey = BitcoinPublicKey::new_uncompressed(public_key);
    let uncompressed_addr = Address::p2pkh(&uncompressed_pubkey, Network::Bitcoin);
    addresses.push(uncompressed_addr.to_string());

    // Generate Bech32 addresses (P2WPKH) - only if successful
    if let Ok(compressed_bech32) = Address::p2wpkh(&compressed_pubkey, Network::Bitcoin) {
        addresses.push(compressed_bech32.to_string());
    }

    Ok(addresses)
}

fn private_key_to_bytes(private_key: &BigUint) -> Result<[u8; 32]> {
    let bytes = private_key.to_bytes_be();

    if bytes.len() > 32 {
        return Err(anyhow!("Private key too large"));
    }

    let mut result = [0u8; 32];
    let start_idx = 32 - bytes.len();
    result[start_idx..].copy_from_slice(&bytes);

    Ok(result)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_private_key() {
        // Test with a known private key from Bitcoin puzzle #1
        let private_key = BigUint::from(1u32);
        let addresses = private_key_to_addresses(&private_key).unwrap();

        println!("Private key 1 addresses: {:?}", addresses);

        // The first Bitcoin puzzle address
        assert!(
            addresses.contains(&"1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH".to_string())
                || addresses.contains(&"1EHNa6Q4Jz2uvNExL497mE43ikXhwF6kZm".to_string())
        );
    }

    #[test]
    fn test_puzzle_3() {
        // Test with Bitcoin puzzle #3 (private key 7)
        let private_key = BigUint::from(7u32);
        let addresses = private_key_to_addresses(&private_key).unwrap();

        println!("Private key 7 addresses: {:?}", addresses);

        // Check if it contains the puzzle #3 address
        assert!(addresses.contains(&"1CUTxxqJWs9FMMSqZgJH6jWNKbKZjNMFLP".to_string()));
    }

    #[test]
    fn test_parse_hex_key() {
        assert_eq!(parse_hex_key("0x1").unwrap(), BigUint::from(1u32));
        assert_eq!(parse_hex_key("1").unwrap(), BigUint::from(1u32));
        assert_eq!(parse_hex_key("0xFF").unwrap(), BigUint::from(255u32));
        assert_eq!(parse_hex_key("ff").unwrap(), BigUint::from(255u32));
    }
}
