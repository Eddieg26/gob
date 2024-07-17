use std::{collections::HashSet, hash::Hash};

pub trait IntoBytes: Sized {
    fn into_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Option<Self>;
}

impl IntoBytes for usize {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut buf = [0; 8];
        buf.copy_from_slice(&bytes);
        Some(usize::from_le_bytes(buf))
    }
}

impl IntoBytes for u64 {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut buf = [0; 8];
        buf.copy_from_slice(&bytes);
        Some(u64::from_le_bytes(buf))
    }
}

impl<I: IntoBytes + Eq + Hash> IntoBytes for HashSet<I> {
    fn into_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&(self.len()).into_bytes());
        for item in self {
            let item_bytes = item.into_bytes();
            bytes.extend(item_bytes.len().into_bytes());
            bytes.extend(item_bytes);
        }
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut set = HashSet::new();
        let mut bytes = bytes;
        let len = usize::from_bytes(&bytes[0..8])?;
        bytes = &bytes[8..];
        for _ in 0..len {
            let item_len = usize::from_bytes(&bytes[0..8])?;
            bytes = &bytes[8..];
            let item = I::from_bytes(&bytes[0..item_len])?;
            bytes = &bytes[item_len..];
            set.insert(item);
        }
        Some(set)
    }
}
