use std::io::{Read, Write};

use arith::{FieldSerde, FieldSerdeResult};

use crate::{Leaf, Node, Path, RangePath, Tree, LEAF_BYTES, LEAF_HASH_BYTES};

impl FieldSerde for Leaf {
    const SERIALIZED_SIZE: usize = LEAF_BYTES;

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        writer.write_all(&self.data)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let mut data = [0u8; LEAF_BYTES];
        reader.read_exact(&mut data)?;
        Ok(Leaf { data })
    }
}

impl FieldSerde for Node {
    const SERIALIZED_SIZE: usize = LEAF_HASH_BYTES;

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        writer.write_all(self.as_bytes())?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let mut data = [0u8; LEAF_HASH_BYTES];
        reader.read_exact(&mut data)?;
        Ok(Node { data })
    }
}

impl FieldSerde for Path {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        self.index.serialize_into(&mut writer)?;
        self.path_nodes.serialize_into(&mut writer)?;
        self.leaf.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let index = usize::deserialize_from(&mut reader)?;
        let path_nodes: Vec<Node> = Vec::deserialize_from(&mut reader)?;
        let leaf = Leaf::deserialize_from(&mut reader)?;

        Ok(Path {
            index,
            path_nodes,
            leaf,
        })
    }
}

impl FieldSerde for RangePath {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        self.left.serialize_into(&mut writer)?;
        self.right.serialize_into(&mut writer)?;
        self.path_nodes.serialize_into(&mut writer)?;
        self.leaves.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let left = usize::deserialize_from(&mut reader)?;
        let right = usize::deserialize_from(&mut reader)?;
        let path_nodes: Vec<Node> = Vec::deserialize_from(&mut reader)?;
        let leaves: Vec<Leaf> = Vec::deserialize_from(&mut reader)?;

        Ok(RangePath {
            left,
            right,
            path_nodes,
            leaves,
        })
    }
}

impl FieldSerde for Tree {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        self.nodes.serialize_into(&mut writer)?;
        self.leaves.serialize_into(&mut writer)
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let nodes = Vec::deserialize_from(&mut reader)?;
        let leaves = Vec::deserialize_from(&mut reader)?;

        Ok(Self { nodes, leaves })
    }
}
