import json
import shutil
import subprocess
import sys
import time

import pymuta

user0 = pymuta.user0
user1 = pymuta.user1
user2 = pymuta.user2
user3 = pymuta.user3

client = pymuta.Client(pymuta.c_server)
ensure_blank_chain = 0

if shutil.which('cita-cli'):
    c_cita_cli = 'cita-cli'
else:
    c_cita_cli = '/src/cita-cli/target/debug/cita-cli.exe'

if ensure_blank_chain and client.get_balance(user1, '0x00') != 0x400000000000000000:
    raise Exception('Ensure you are on a blank chain')


def call(command):
    print('$', command)
    c, r = subprocess.getstatusoutput(command)
    if c != 0:
        sys.exit(r)
    print(r)
    return json.loads(r)['result']


prefix = f'{c_cita_cli} rpc --url {pymuta.c_server} --no-color'


def test_peer_count():
    r = call(f'{prefix} peerCount')
    assert r == 42


def test_block_number():
    r0 = call(f'{prefix} blockNumber')
    time.sleep(6)
    r1 = call(f'{prefix} blockNumber')
    assert r1 > r0


def test_transfer_balance():
    b_user0_0 = client.get_balance(user0)
    b_user1_0 = client.get_balance(user1)
    b_user2_0 = client.get_balance(user2)

    call(
        f'{prefix} sendRawTransaction --private-key {user1.private_key} --address {user2.address} --value 10 --code 0x')
    time.sleep(6)

    b_user0_1 = client.get_balance(user0)
    b_user1_1 = client.get_balance(user1)
    b_user2_1 = client.get_balance(user2)

    assert b_user0_1 == b_user0_0 + 21000
    assert b_user1_1 == b_user1_0 - 21000 - 10
    assert b_user2_1 == b_user2_0 + 10


def test_get_block_by_hash():
    b0 = call(f'{prefix} getBlockByNumber --height latest')
    h = b0['hash']
    b1 = call(f'{prefix} getBlockByHash --hash {h}')
    assert b0 == b1


def test_get_block_by_number():
    tx = client.sign_tx(user1.private_key, user2.address, "", 10, 100000)
    receipt = client.sync_raw_transaction(tx)
    block_number = receipt['blockNumber']
    tx_hash = receipt['transactionHash']

    b = call(f'{prefix} getBlockByNumber --height {block_number}')
    assert len(b['body']['transactions']) == 1
    assert b['body']['transactions'][0] == tx_hash

    b = call(f'{prefix} getBlockByNumber --height {block_number}')
    assert len(b['body']['transactions']) == 1
    assert b['body']['transactions'][0]['from'] == user1.address
    assert b['body']['transactions'][0]['hash'] == tx_hash


def test_get_block_include_tx_with_data():
    tx = client.sign_tx(user1.private_key, user2.address, "0x1234", 10, 100000)
    r = client.sync_raw_transaction(tx)
    block_number = r['blockNumber']
    block = call(f'{prefix} getBlockByNumber --height {block_number} --with-txs')
    txs = block['body']['transactions']
    assert len(txs) == 1
    assert txs[0]['content'] == [18, 52]
    assert txs[0]['from'] == '0x2ae83ce578e4bb7968104b5d7c034af36a771a35'
    assert int(block['header']['quotaUsed'], 16) == 21000 + 68 + 68


def test_get_logs():
    pass


def test_call():
    ss = pymuta.SimpleStorage(client, user0)
    ss.deploy()
    ss.set(42)
    r = call(f'{prefix} call --from {user1.address} --to {ss.address} --data 0x6d4ce63c')
    assert r == '0x000000000000000000000000000000000000000000000000000000000000002a'
    ss.set(15)
    r = call(f'{prefix} call --from {user1.address} --to {ss.address} --data 0x6d4ce63c')
    assert r == '0x000000000000000000000000000000000000000000000000000000000000000f'


def test_get_transaction():
    pass


def test_get_code():
    ss = pymuta.SimpleStorage(client, user0)
    ss.deploy()
    r = call(f'{prefix} getCode --address {ss.address}')
    assert len(r) == 223


def test_get_abi():
    pass


def test_get_block_header():
    call(f'{prefix} getBlockHeader --height latest')


def test_get_storage_at():
    ss = pymuta.SimpleStorage(client, user0)
    ss.deploy()
    ss.set(42)
    v = call(f'{prefix} getStorageAt --address {ss.address} --height latest --key 0x0000000000000000000000000000000000000000000000000000000000000000')
    assert v[-1] == 42


if __name__ == '__main__':
    test_peer_count()
    test_block_number()
    test_transfer_balance()
    test_get_block_by_hash()
    # test_get_block_by_number()
    test_get_block_include_tx_with_data()
    test_get_logs()
    test_call()
    # test_get_transaction()
    test_get_code()
    test_get_abi()
    test_get_block_header()
    test_get_storage_at()
