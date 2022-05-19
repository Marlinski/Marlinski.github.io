<script>
    import { tokenPrice, maxSupply } from "../utils/ContractApi.js";
    import IpfsImage from "./IpfsImage.svelte";
    import SaleStatus from "./SaleStatus.svelte";
    import MintButton from "./MintButton.svelte";
	import TokensOfSection from "../components/TokensOfSection.svelte";
    
    let gridHeight;
    $: gridHeight;
</script>

<IpfsImage />
<div id="statusSection">
    <div id="mintSection" bind:clientHeight={gridHeight}>
        <SaleStatus />
        <MintButton/>
        <TokensOfSection />
    </div>
</div>

<style>
    #mintSection {
        margin: 0 auto;
        margin-top: 15px;
        display: flex;
        flex-direction: column;
        align-items: center;
        width: fit-content;
        width: -moz-fit-content;
        padding-top: 20px;
        padding-bottom: 20px;
    }
</style>
