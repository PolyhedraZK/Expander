use std::io::{Read, Write};

use serdes::{ExpSerde, SerdeResult};

use crate::{Leaf, Node, Path, RangePath, Tree, LEAF_BYTES, LEAF_HASH_BYTES};

impl ExpSerde for Leaf {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        writer.write_all(&self.data)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let mut data = [0u8; LEAF_BYTES];
        reader.read_exact(&mut data)?;
        Ok(Leaf { data })
    }
}

impl ExpSerde for Node {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        writer.write_all(self.as_bytes())?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let mut data = [0u8; LEAF_HASH_BYTES];
        reader.read_exact(&mut data)?;
        Ok(Node { data })
    }
}

impl ExpSerde for Path {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        <Vec<Node> as ExpSerde>::serialize_into(&self.path_nodes, &mut writer)?;
        self.leaf.serialize_into(&mut writer)?;
        self.index.serialize_into(&mut writer)?;

        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let path_nodes: Vec<Node> = <Vec<Node> as ExpSerde>::deserialize_from(&mut reader)?;
        let leaf = Leaf::deserialize_from(&mut reader)?;
        let index = usize::deserialize_from(&mut reader)?;

        Ok(Path {
            path_nodes,
            leaf,
            index,
        })
    }
}

impl ExpSerde for RangePath {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.left.serialize_into(&mut writer)?;
        self.right.serialize_into(&mut writer)?;
        <Vec<Node> as ExpSerde>::serialize_into(&self.path_nodes, &mut writer)?;
        self.leaves.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let left = usize::deserialize_from(&mut reader)?;
        let right = usize::deserialize_from(&mut reader)?;
        let path_nodes: Vec<Node> = <Vec<Node> as ExpSerde>::deserialize_from(&mut reader)?;
        let leaves: Vec<Leaf> = <Vec<Leaf> as ExpSerde>::deserialize_from(&mut reader)?;

        Ok(RangePath {
            left,
            right,
            path_nodes,
            leaves,
        })
    }
}

impl ExpSerde for Tree {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        <Vec<Node> as ExpSerde>::serialize_into(&self.nodes, &mut writer)?;
        self.leaves.serialize_into(&mut writer)
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let nodes: Vec<Node> = <Vec<Node> as ExpSerde>::deserialize_from(&mut reader)?;
        let leaves: Vec<Leaf> = <Vec<Leaf> as ExpSerde>::deserialize_from(&mut reader)?;

        Ok(Self { nodes, leaves })
    }
}
