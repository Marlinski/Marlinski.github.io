<script>
	import { connected, selectedAccount } from "svelte-web3";
    import LoginWeb3 from "../components/LoginWeb3.svelte";
	import MintSection from "../components/MintSection.svelte";
</script>

<svelte:head>
	<title>Codelab NFT</title>
</svelte:head>

<div>
    {#if $connected && $selectedAccount}
		<MintSection />
	{:else}
		<div id="connect">
			<LoginWeb3 />
		</div>
    {/if}
</div>

<style>
    #connect {
        margin: auto;
		width: fit-content;
		padding-top:50px;
    }
</style>

