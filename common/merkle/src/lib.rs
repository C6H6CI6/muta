use merkle_cbt::{MerkleProof as CBMTMerkleProof, MerkleTree as CBMTMerkleTree, CBMT};

use protocol::{types::Hash, Bytes};

struct MergeHash {}

impl merkle_cbt::merkle_tree::Merge for MergeHash {
    type Item = Hash;

    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let left = left.as_bytes();
        let right = right.as_bytes();

        let mut root = Vec::with_capacity(left.len() + right.len());
        root.extend_from_slice(&left);
        root.extend_from_slice(&right);
        Hash::digest(Bytes::from(root))
    }
}

type CBMTHash = CBMT<Hash, MergeHash>;
type CBMTHashMerkleTree = CBMTMerkleTree<Hash, MergeHash>;
type CBMTHashProof = CBMTMerkleProof<Hash, MergeHash>;

pub struct Proof {
    inner: CBMTHashProof,
}

impl Proof {
    pub fn new(lemmas: Vec<Hash>, indices: Vec<u32>) -> Self {
        let proof = CBMTHashProof::new(indices, lemmas);
        Self { inner: proof }
    }

    pub fn verify(&self, root: &Hash, leaves: &[Hash]) -> bool {
        self.inner.verify(root, leaves)
    }

    pub fn lemmas(&self) -> &[Hash] {
        self.inner.lemmas()
    }

    pub fn indices(&self) -> &[u32] {
        self.inner.indices()
    }
}

pub struct Merkle {
    tree: CBMTHashMerkleTree,
}

impl Merkle {
    pub fn from_hashes(hashes: Vec<Hash>) -> Self {
        Merkle {
            tree: CBMTHash::build_merkle_tree(hashes),
        }
    }

    pub fn get_root_hash(&self) -> Hash {
        self.tree.root()
    }

    pub fn get_proof(&self, indices: &[u32]) -> Option<Proof> {
        self.tree
            .build_proof(indices)
            .and_then(|p| Some(Proof::new(p.lemmas().to_vec(), p.indices().to_vec())))
    }
}

#[test]
fn test_proof() {
    let hashes = vec![
        "0x6b5f8ec6fc0fc39629d7aff3714acb9b644cad4812d44730ce69908b3688161e",
        "0x431ee41c5756b937610c8c274d94c666bc77e336410648019a80bf2912a71c7b",
        "0xe57d559ee8a2b49cff1781be890a225d2b674c98a15a8e5708afc23562c93ce1",
        "0x95786c8f3310677bd065cd1e76d597d091085b460f742007112e16f44c0d3bee",
        "0x8283fe529446477378c4cfbdfab7ee9eef9b21c1e2080f43990720d221389003",
        "0xb6afcc6c3baa9a924ca6cab79a3ed6ef50930462bfe87f1e90a1ca6e28c67e55",
        "0x14c5662b1cac443879343b273d9d8a966a516a13bde650048d6fa709922dd8e0",
        "0xb305168b5f7b96e26ac8c7bc85ea6d40199e557505ab475e0e3b537ea25236df",
        "0x94f40b1cb5dc617b9c9c9ae97cd9927692a8ee1ea60590320f885661822bbdce",
        "0x7946ef09bd8e7b32e6d0cf1d781bb8aa2219b9e84db1005b3e483017a4e605ff",
        "0x9b1541cceb70313e623b528f0029b6d16b4db2cb2277d0d14e62c00a6909e74c",
        "0x244a43630cbeb1a24403c7a11a1f9e55b6c1e1e087c58579e237079dd9466cdd",
        "0x7a5e6c89b02a007c9c4075f35343bfd5633890491f651634e33f7e1c38f5f013",
    ];
    let leaves: Vec<Hash> = hashes
        .into_iter()
        .map(|r| Hash::from_hex(r).unwrap())
        .collect();

    let tree = Merkle::from_hashes(leaves.clone());
    let root = tree.get_root_hash();

    // build proof
    let proof = tree.get_proof(&[0, 3]).unwrap();
    let indices = proof.indices();
    let lemmas = proof.lemmas();

    println!("indices {:?} lemmas {:?}", indices, lemmas);
    // rebuild proof
    // let mut new_leaves = vec![];
    // for i in indices.iter() {
    //     new_leaves.push(tree.nodes()[*i as usize].clone());
    // }

    let proof = Proof::new(lemmas.to_vec(), indices.to_vec());
    let leaves: Vec<Hash> = indices
        .iter()
        .map(|i| leaves[*i as usize].clone())
        .collect();
    assert_eq!(proof.verify(&root, &leaves), true);
    // assert_eq!(root, root2);
    // let proof = CBMTI32::build_merkle_proof(lemmas, &indices).unwrap();
    // proof.verify(root: &T, leaves: &[T])
    // let indices = vec![0usize, 5];
    // let proof_leaves = indices
    //     .iter()
    //     .map(|i| leaves[*i as usize].clone())
    //     .collect::<Vec<_>>();
    // let proof = CBMTI32::build_merkle_proof(&leaves, &indices).unwrap();
    //
    // assert_eq!(vec![11, 3, 2], proof.lemmas());
    // assert_eq!(Some(1), proof.root(&proof_leaves));
    //
    // // merkle proof for single leaf
    // let leaves = vec![2i32];
    // let indices = vec![0usize];
    // let proof_leaves = indices
    //     .iter()
    //     .map(|i| leaves[*i as usize].clone())
    //     .collect::<Vec<_>>();
    // let proof = CBMTI32::build_merkle_proof(&leaves, &indices).unwrap();
    // assert!(proof.lemmas().is_empty());
    // assert_eq!(Some(2), proof.root(&proof_leaves));
    // use protocol::types::Hash;
    //
    // use crate::Merkle;
    //
    // let mut proot_path = vec![
    //     "0x6b5f8ec6fc0fc39629d7aff3714acb9b644cad4812d44730ce69908b3688161e",
    //     "0x431ee41c5756b937610c8c274d94c666bc77e336410648019a80bf2912a71c7b",
    //     "0xe57d559ee8a2b49cff1781be890a225d2b674c98a15a8e5708afc23562c93ce1",
    //     "0x95786c8f3310677bd065cd1e76d597d091085b460f742007112e16f44c0d3bee",
    //     "0x8283fe529446477378c4cfbdfab7ee9eef9b21c1e2080f43990720d221389003",
    //     "0xb6afcc6c3baa9a924ca6cab79a3ed6ef50930462bfe87f1e90a1ca6e28c67e55",
    //     "0x14c5662b1cac443879343b273d9d8a966a516a13bde650048d6fa709922dd8e0",
    //     "0xb305168b5f7b96e26ac8c7bc85ea6d40199e557505ab475e0e3b537ea25236df",
    //     "0x94f40b1cb5dc617b9c9c9ae97cd9927692a8ee1ea60590320f885661822bbdce",
    //     "0x7946ef09bd8e7b32e6d0cf1d781bb8aa2219b9e84db1005b3e483017a4e605ff",
    //     "0x9b1541cceb70313e623b528f0029b6d16b4db2cb2277d0d14e62c00a6909e74c",
    //     "0x244a43630cbeb1a24403c7a11a1f9e55b6c1e1e087c58579e237079dd9466cdd",
    //     "0x7a5e6c89b02a007c9c4075f35343bfd5633890491f651634e33f7e1c38f5f013",
    // ];
    //
    // // let mut proot_path = vec![
    // //     "0xde9eb75ceb87ef28129958af7ff9867daf3c1c0ff1f2fd0863af4aba6ce298cd"
    // , //     "0x6b5f8ec6fc0fc39629d7aff3714acb9b644cad4812d44730ce69908b3688161e"
    // , // ];
    //
    // let proof_path: Vec<Hash> = proot_path
    //     .into_iter()
    //     .map(|r| Hash::from_hex(r).unwrap())
    //     .collect();
    //
    // // let root = Merkle::from_hashes(proof_path.clone())
    // //     .get_root_hash()
    // //     .unwrap();
    // let proof = Merkle::from_hashes(proof_path)
    //     .get_proof_by_input_index(0)
    //     .unwrap();
    //
    // println!("{:?}", proof);
    // let left =
    // Hash::from_hex("
    // 0xde9eb75ceb87ef28129958af7ff9867daf3c1c0ff1f2fd0863af4aba6ce298cd")
    //     .unwrap()
    //     .as_bytes();
    // println!(
    //     "left {:?}",
    //     Hash::from_hex("
    // 0xde9eb75ceb87ef28129958af7ff9867daf3c1c0ff1f2fd0863af4aba6ce298cd")
    //         .unwrap()
    //         .as_hex()
    // );
    // let right =
    //     Hash::from_hex("
    // 0x6b5f8ec6fc0fc39629d7aff3714acb9b644cad4812d44730ce69908b3688161e")
    //         .unwrap()
    //         .as_bytes();
    // let mut root = Vec::with_capacity(left.len() + right.len());
    //
    // root.extend_from_slice(&left);
    // root.extend_from_slice(&right);
    // let root = Hash::digest(Bytes::from(root));
    // println!("root {:?}", root.as_hex());
    // // list.reverse();
    //
    // let hashes: Vec<Hash> = list
    //     .into_iter()
    //     .map(|r| Hash::from_hex(r).unwrap())
    //     .collect();
    //
    // let root = Merkle::from_hashes(hashes).get_root_hash().unwrap();

    // println!("success {:?}", root.as_hex());
}
