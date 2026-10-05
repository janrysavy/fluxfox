//! Strict snapshot decoding without changing ordinary native tree serde.
use super::{FoxHashMap, FoxTreeMap, FoxTreeNode};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tree<U> {
    nodes: Vec<Node<U>>,
    name_to_index: FoxHashMap<String, usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Node<U> {
    name: String,
    index: usize,
    parent: usize,
    children: Vec<usize>,
    data: U,
}

pub(crate) fn deserialize_tree<'de, D, T, U>(deserializer: D) -> Result<FoxTreeMap<T>, D::Error>
where
    D: Deserializer<'de>,
    U: Deserialize<'de> + Into<T>,
{
    let tree = Tree::<U>::deserialize(deserializer)?;
    Ok(FoxTreeMap {
        nodes: tree
            .nodes
            .into_iter()
            .map(|node| FoxTreeNode {
                name: node.name,
                index: node.index,
                parent: node.parent,
                children: node.children,
                data: node.data.into(),
            })
            .collect(),
        name_to_index: tree.name_to_index,
    })
}
