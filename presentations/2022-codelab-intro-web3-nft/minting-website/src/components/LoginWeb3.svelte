<script>
    import {
        defaultChainStore,
    } from "svelte-web3";

    import metamask from "images/metamask.png";

    let metamaskInstalled = true;
    const enableBrowser = () => {
        defaultChainStore.setBrowserProvider().catch(err => {
            metamaskInstalled = false;
        });
    }
</script>

{#if metamaskInstalled}
    <div id="login" on:click={enableBrowser}>
        <span><img src={metamask} alt="metamask"/></span>
        Connect with Metamask
    </div>
{:else}
    <a href="https://metamask.io/" target="_blank"><div id="login">install Metamask first!</div></a>
{/if}

<style>
    #login {
        display: flex;
        align-items: center;
        background-color: white;
        border: 1px solid grey;
        border-radius: 8px;
        font-weight: 400;
        cursor: pointer;
        transition: 0.3s;
        padding: 5px;
    }

    img {
        vertical-align:bottom;
        width:40px;
        height:40px;
        padding: auto;
        padding-right: 5px;
    }

    #login:hover {
        background-color: slategrey;
        font-weight: 400;
        color: white;
        transition: 0.3s;
        border: 1px solid white;
    }
</style>
