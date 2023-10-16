#[cfg(feature = "fast_hash")]
use hashbrown;
#[cfg(not(feature = "fast_hash"))]
use std::collections::{HashMap, HashSet};

use multimap::MultiMap;

#[cfg(feature = "fast_hash")]
pub type CustomHashMap<K, V> = hashbrown::HashMap<K, V>;
#[cfg(feature = "fast_hash")]
pub type CustomHashSet<K> = hashbrown::HashSet<K>;
#[cfg(feature = "fast_hash")]
pub type CustomMultiHashMap<K, V> = MultiMap<K, V, hashbrown::hash_map::DefaultHashBuilder>;

// default implementations
#[cfg(not(feature = "fast_hash"))]
pub type CustomMultiHashMap<K, V> = MultiMap<K, V>;
#[cfg(not(feature = "fast_hash"))]
pub type CustomHashMap<K, V> = HashMap<K, V>;
#[cfg(not(feature = "fast_hash"))]
pub type CustomHashSet<K> = HashSet<K>;
