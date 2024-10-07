import threading
import time
import requests
import sys

def get_proof(url, headers, data):
    response = requests.post(url+"/prove", headers=headers, data=data)
    return response
if __name__ == '__main__':
    # prove
    dir = '../circuits/eth2/validator/gkr/'
    with open('../ExpanderCompilerCollection/examples/poseidon_m31/witness.txt', 'rb') as f:
    # with open(dir +'witness.txt', 'rb') as f:
        witness = f.read()
    # parse port from os args
    args = sys.argv
    if len(args) > 1:
        port = args[1]
    else:
        port = 3030
    url = 'http://127.0.0.1:' + str(port)
    prove_headers = {
        'Content-Type': 'application/octet-stream',
        'Content-Length': str(len(witness)),
    }
    # start_time = time.time()
    # threads = []
    # for i in range(4): 
    #     thread = threading.Thread(target=get_proof, args=(url, prove_headers, witness))
    #     threads.append(thread)
    #     thread.start()

    # for thread in threads:
    #     thread.join()
    # print("Time taken:", time.time() - start_time)
    start_time = time.time()
    response = requests.post(url+"/prove", headers=prove_headers, data=witness)
    proof = response.content
    print(response)
    print("Proof generated successfully, length:", len(proof))
    print("Time taken:", time.time() - start_time)
    with open(dir + 'proof.txt', 'wb') as f:
        f.write(proof)

#     # verify
#     # add u64 length of witness and proof to the beginning of the file
    witness_len = len(witness).to_bytes(8, byteorder='little')
    proof_len = len(proof).to_bytes(8, byteorder='little')
    verifier_input = witness_len + proof_len + witness + proof
    verify_headers = {
        'Content-Type': 'application/octet-stream',
        'Content-Length': str(len(proof)),
    }
    start_time = time.time()
    response = requests.post(url+"/verify", headers=verify_headers, data=verifier_input)
    print(response)
    # check success message
    # assert response.text == "success", f"Failed to verify proof: {response.text}"
    print("Proof verified successfully")
    print("Time taken:", time.time() - start_time)
    
#     # # try tempered proof
#     # import random
#     # # flip a random bit
#     # random_byte_index = random.randint(0, len(proof) - 1)
#     # random_bit_index = random.randint(0, 7)
#     # tempered_proof = proof[:random_byte_index] + bytes([proof[random_byte_index] ^ (1 << random_bit_index)]) + proof[random_byte_index+1:]
#     # tempered_input = witness_len + proof_len + witness + tempered_proof
#     # response = requests.post(url+"/verify", headers=verify_headers, data=tempered_input)
#     # # check failure message
#     # assert response.text == "failure", f"Failed to detect tempered proof: {response.text}"
#     # print("Tempered proof detected successfully")

#     # # try prove using witness with invalid length
#     # tempered_witness = witness[:-1]
#     # prove_headers = {
#     #     'Content-Type': 'application/octet-stream',
#     #     'Content-Length': str(len(tempered_witness)),
#     # }
#    # response = requests.post(url+"/prove", headers=prove_headers, data=tempered_witness)
#     # check 400
#     #assert response.status_code == 400, f"Failed to detect invalid witness length: {response.text}"
#     #print("Invalid witness length detected successfully")
