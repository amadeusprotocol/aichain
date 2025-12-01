use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DST_TX: &[u8] = b"AMADEUS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_TX_";

mod args_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(args: &[Vec<u8>], ser: S) -> Result<S::Ok, S::Error> {
        let v: Vec<serde_bytes::ByteBuf> =
            args.iter().map(|a| serde_bytes::ByteBuf::from(a.clone())).collect();
        v.serialize(ser)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<Vec<u8>>, D::Error> {
        let v: Vec<serde_bytes::ByteBuf> = Deserialize::deserialize(de)?;
        Ok(v.into_iter().map(|b| b.into_vec()).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxAction {
    #[serde(with = "args_serde")]
    pub args: Vec<Vec<u8>>,
    pub contract: String,
    pub function: String,
    pub op: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_symbol: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_amount: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tx {
    pub action: TxAction,
    pub nonce: i128,
    #[serde(with = "serde_bytes")]
    pub signer: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxU {
    #[serde(with = "serde_bytes")]
    pub hash: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
    pub tx: Tx,
}

fn get_public_key(sk_bytes: &[u8]) -> Result<Vec<u8>, &'static str> {
    use bls12_381::Scalar;
    use group::Curve;

    if sk_bytes.len() != 64 {
        return Err("secret key must be 64 bytes");
    }
    let bytes_64: [u8; 64] = sk_bytes.try_into().map_err(|_| "invalid sk length")?;
    let sk_scalar = Scalar::from_bytes_wide(&bytes_64);
    let pk_g1 = bls12_381::G1Projective::generator() * sk_scalar;
    Ok(pk_g1.to_affine().to_compressed().to_vec())
}

fn sign(sk_bytes: &[u8], message: &[u8], dst: &[u8]) -> Result<Vec<u8>, &'static str> {
    use bls12_381::Scalar;

    if sk_bytes.len() != 64 {
        return Err("secret key must be 64 bytes");
    }
    let bytes_64: [u8; 64] = sk_bytes.try_into().map_err(|_| "invalid sk length")?;
    let sk_scalar = Scalar::from_bytes_wide(&bytes_64);
    let mut sk_be = sk_scalar.to_bytes();
    sk_be.reverse();
    let sk = blst::min_pk::SecretKey::from_bytes(&sk_be).map_err(|_| "invalid secret key")?;
    let sig = sk.sign(message, dst, &[]);
    Ok(sig.to_bytes().to_vec())
}

pub fn build(
    sk_bytes: &[u8],
    contract: &str,
    function: &str,
    args: &[Vec<u8>],
    attached_symbol: Option<&[u8]>,
    attached_amount: Option<&[u8]>,
) -> Result<Vec<u8>, &'static str> {
    let pk = get_public_key(sk_bytes)?;
    let nonce = js_sys::Date::now() as i128 * 1_000_000;

    let action = TxAction {
        op: "call".to_string(),
        contract: contract.to_string(),
        function: function.to_string(),
        args: args.to_vec(),
        attached_symbol: attached_symbol.map(|s| s.to_vec()),
        attached_amount: attached_amount.map(|a| a.to_vec()),
    };

    let tx = Tx { signer: pk, nonce, action };
    let tx_encoded = vecpak::to_vec(&tx).map_err(|_| "failed to encode tx")?;
    let hash: [u8; 32] = Sha256::digest(&tx_encoded).into();
    let signature = sign(sk_bytes, &hash, DST_TX)?;

    let txu = TxU { hash: hash.to_vec(), signature, tx };
    vecpak::to_vec(&txu).map_err(|_| "failed to encode txu")
}

#[allow(dead_code)]
pub fn build_mint_tx(sk_bytes: &[u8], symbol: &str, amount: i128) -> Result<Vec<u8>, &'static str> {
    build(
        sk_bytes,
        "Coin",
        "mint",
        &[symbol.as_bytes().to_vec(), amount.to_string().as_bytes().to_vec()],
        None,
        None,
    )
}

pub fn build_transfer_tx(
    sk_bytes: &[u8],
    receiver: &[u8],
    symbol: &str,
    amount: i128,
) -> Result<Vec<u8>, &'static str> {
    build(
        sk_bytes,
        "Coin",
        "transfer",
        &[
            receiver.to_vec(),
            amount.to_string().as_bytes().to_vec(),
            symbol.as_bytes().to_vec(),
        ],
        None,
        None,
    )
}
