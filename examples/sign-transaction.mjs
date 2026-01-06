import { bls12_381 } from '@noble/curves/bls12-381.js';
import { bytesToNumberLE, numberToBytesBE, hexToBytes } from '@noble/curves/utils.js';
import bs58 from 'bs58';

const [,, sk64B58, contract, func, argsJson, network = 'mainnet'] = process.argv;

if (!sk64B58 || !contract || !func || !argsJson) {
  console.error('Usage: node sign-transaction.mjs <sk_base58> <contract> <function> <args_json> [network]');
  console.error('Example: node sign-transaction.mjs SK_B58 Coin transfer \'[{"b58":"RECIPIENT"},"1000000000","AMA"]\' testnet');
  process.exit(1);
}

const sk64 = bs58.decode(sk64B58);
const skScalar = bytesToNumberLE(sk64) % bls12_381.fields.Fr.ORDER;
const privateKey = numberToBytesBE(skScalar, 32);
const blsl = bls12_381.longSignatures;
const publicKey = blsl.getPublicKey(privateKey);
const signer = bs58.encode(publicKey.toBytes(true));

const args = JSON.parse(argsJson);

const mcpReq = {
  jsonrpc: '2.0',
  id: 1,
  method: 'tools/call',
  params: { name: 'create_transaction', arguments: { signer, contract, function: func, args } }
};

const response = await fetch('https://mcp.ama.one', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(mcpReq)
});

const data = await response.json();
if (data.error) {
  console.error('MCP Error:', data.error);
  process.exit(1);
}
const result = JSON.parse(data.result.content[0].text);
const signingHash = hexToBytes(result.signing_payload);

const DST = 'AMADEUS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_TX_';
const msgPoint = blsl.hash(signingHash, DST);
const signature = blsl.sign(msgPoint, privateKey);
const signatureB58 = bs58.encode(signature.toBytes(true));

const submitReq = {
  jsonrpc: '2.0',
  id: 2,
  method: 'tools/call',
  params: { name: 'submit_transaction', arguments: { transaction: result.blob, signature: signatureB58, network } }
};

const submitResp = await fetch('https://mcp.ama.one', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(submitReq)
});

const submitData = await submitResp.json();
if (submitData.error) {
  console.error('Submit Error:', submitData.error);
  process.exit(1);
}
console.log(JSON.parse(submitData.result.content[0].text));
