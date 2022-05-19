<script>
  import { ipfsUri } from "../utils/ContractApi.js";
</script>

<div id="nft">
    <a href="{$ipfsUri}"><img alt="" src="{$ipfsUri}"/></a>
</div>

<style>
  #nft {
      border: 1px solid grey;
      border-radius: 8px;
      box-shadow: 5px 5px 20px grey;
      margin: auto;
      margin-top:5px;
      margin-bottom:5px;
      width: fit-content;
  }

  img {
    width: 250px;
    height: 250px;
  }
</style>