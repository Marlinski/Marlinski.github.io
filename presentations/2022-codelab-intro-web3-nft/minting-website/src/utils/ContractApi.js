import { get, readable, derived } from 'svelte/store';
import { web3, selectedAccount, connected, makeContractStore } from 'svelte-web3';
import nftJSON from "../contracts/NFTCollection.json";
import minterJSON from "../contracts/MinterFactory.json";

export const nftAddress = nftJSON.address;
export const minterAddress = minterJSON.address;
export const nftContract = makeContractStore(nftJSON.abi, nftJSON.address);
export const minterContract = makeContractStore(minterJSON.abi, minterJSON.address);

const tick = readable(new Date(), function start(set) {
	const interval = setInterval(() => {
		set(new Date());
	}, 2000);

	return function stop() {
		clearInterval(interval);
	};
});

export const updatedBalance = derived(
	[web3, connected, selectedAccount, tick],
	([web3, connected, selectedAccount, tick], set) => {
		if (web3 && connected && selectedAccount) {
			web3.eth.getBalance(selectedAccount).then(set);
		} else {
			set("0");
		}
	}, 0);


export const balanceOfConnectedAccount = derived(
	[nftContract, connected, selectedAccount, tick],
	([nftContract, connected, selectedAccount, tick], set) => {
		if (nftContract && connected && selectedAccount) {
			nftContract.methods.balanceOf(selectedAccount).call().then(set);
		} else {
			set([]);
		}
	}, []);

export const getCurrentBlock = derived(
	[web3, tick],
	([web3, tick], set) => {
		if (web3) {
			web3.eth.getBlockNumber().then(set);
		}
	}, 0);

export const totalMinted = derived(
	[nftContract, tick],
	([nftContract, tick], set) => {
		if (nftContract) {
			nftContract.methods.totalSupply().call().then(set)
		}
	}, 0);

export const tokenPriceWei = derived(
	[minterContract, web3],
	([minterContract, web3], set) => {
		if (web3 && minterContract) {
			minterContract.methods.mintParams().call().then(it => set(it.tokenPrice));
		}
	}, 0);

export const tokenPrice = derived(
	[tokenPriceWei, web3],
	([tokenPriceWei, web3], set) => {
		if (web3 && tokenPriceWei) {
			set(web3.utils.fromWei(tokenPriceWei, 'ether'));
		}
	}, 0);

export const ipfsUri = derived(
	[nftContract],
	([nftContract], set) => {
		if (nftContract) {
			nftContract.methods.uri().call().then(async metadataUri => {
				let uri = metadataUri.replace('ipfs://','');
				let response = await fetch("https://ipfs.io/ipfs/"+uri);
				let metadata = await response.json();
				let imageuri = "https://ipfs.io/ipfs/"+metadata.image.replace('ipfs://','');
				set(imageuri);
			});
		} else {
			set("");
		}
	}, "");


export const maxSupply = derived(
	minterContract,
	(minterContract, set) => {
		if (minterContract) {
			minterContract.methods.mintParams().call().then(it => set(it.maxSupply))
		}
	}, 0);

export const buildMintTx = async (recipient) => {
	const ctr = await get(minterContract)
	if (ctr == null) {
		return null;
	}
	const contractAddress = ctr._address;
	
	const Web3 = get(web3);
	const tokenPriceBN = new Web3.utils.BN(get(tokenPriceWei));
	const orderPriceWei = tokenPriceBN;
	
	const from = get(selectedAccount);
	const tx = ctr.methods.mintNFT(recipient);
	const gas = await tx.estimateGas({ from: from, value: orderPriceWei });
	const gasPrice = await get(web3).eth.getGasPrice();
	const txData = tx.encodeABI();
	return {
		gas: gas,
		gasPrice: gasPrice,
		from: from,
		to: contractAddress,
		value: orderPriceWei,
		data: txData
	};
};

export const mint = async () => {
	const ctr = await get(minterContract)
	if (ctr == null) {
		return null;
	}

	return await buildMintTx().then(get(web3).eth.sendTransaction);
};