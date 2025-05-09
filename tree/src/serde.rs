use std::io::{Read, Write};

use serdes::{ExpSerde, SerdeResult};

use crate::{Leaf, Node, RangePath, Tree, LEAF_BYTES, LEAF_HASH_BYTES};

impl ExpSerde for Leaf {
    const SERIALIZED_SIZE: usize = LEAF_BYTES;

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
    const SERIALIZED_SIZE: usize = LEAF_HASH_BYTES;

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

impl ExpSerde for RangePath {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.left.serialize_into(&mut writer)?;
        self.path_nodes.serialize_into(&mut writer)?;
        self.leaves.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let left = usize::deserialize_from(&mut reader)?;
        let path_nodes: Vec<Node> = Vec::deserialize_from(&mut reader)?;
        let leaves: Vec<Leaf> = Vec::deserialize_from(&mut reader)?;

        Ok(RangePath {
            left,
            path_nodes,
            leaves,
        })
    }
}

impl ExpSerde for Tree {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.nodes.serialize_into(&mut writer)?;
        self.leaves.serialize_into(&mut writer)
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let nodes: Vec<Node> = Vec::deserialize_from(&mut reader)?;
        let leaves: Vec<Leaf> = Vec::deserialize_from(&mut reader)?;

        Ok(Self { nodes, leaves })
    }
}
