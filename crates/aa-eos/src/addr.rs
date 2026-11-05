//! A player's address on the P2P path.
//!
//! The session names a peer by an IPv4 address and port, and sends those to the
//! other peers, so a peer needs one address that every node computes alike. EOS
//! has only the ProductUserId, so the address is derived from it: 10.x.y.z, a
//! private range no peer of this path would otherwise have, and a port above the
//! reserved ones. Sixteen players of 2^40 addresses make a clash a matter of
//! about one game in ten billion; a clash shows as those two players being one
//! connection, and is logged by the link.
use std::net::{Ipv4Addr, SocketAddrV4};

/// FNV-1a over the ASCII lower case of the ID (IDs are hexadecimal, and their
/// case carries no meaning).
fn hash(user: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in user.bytes() {
        h ^= u64::from(b.to_ascii_lowercase());
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The address of a ProductUserId, the same on every machine.
pub fn of_user(user: &str) -> SocketAddrV4 {
    let h = hash(user);
    let ip = Ipv4Addr::new(10, (h >> 40) as u8, (h >> 32) as u8, (h >> 24) as u8);
    // 1024 ..= 65535
    let port = 1024 + ((h & 0xFFFF) as u32 * (65536 - 1024) / 65536) as u16;
    SocketAddrV4::new(ip, port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_user_is_the_same_address_everywhere() {
        let a = "0002a1b2c3d4e5f60718293a4b5c6d7e";
        assert_eq!(of_user(a), of_user(a));
        assert_eq!(of_user(a), of_user(&a.to_ascii_uppercase()));
        // A pinned value: this rule may not change between versions of the game
        // that play together.
        assert_eq!(of_user(a).to_string(), PINNED);
    }
    const PINNED: &str = "10.104.68.167:38284";

    #[test]
    fn addresses_are_ones_the_session_can_use() {
        for i in 0..20_000u32 {
            let a = of_user(&format!("{i:032x}"));
            assert_ne!(a.port(), 0);
            assert!(a.ip().is_private());
            assert!(!a.ip().is_unspecified() && !a.ip().is_broadcast());
        }
    }

    #[test]
    fn different_users_do_not_meet() {
        let mut seen = std::collections::HashSet::new();
        for i in 0..200_000u32 {
            assert!(seen.insert(of_user(&format!("{:032x}", u128::from(i) * 0x1_0000_0001))), "{i}");
        }
    }
}
