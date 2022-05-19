<script>
	import Connect from "./Connected.svelte";
</script>

<nav>
	<div id="menu">
	</div>

	<div id="connect">
		<Connect />
	</div>
</nav>

<style>
	nav {
		display: flex;
		justify-content: space-between;
		border-bottom: 1px solid grey;
		font-weight: 300;
		padding: 0 1em;
	}

	@media screen and (max-width: 600px) {
		nav {
			flex-flow: row wrap;
			flex-direction: column-reverse;
		}
		#menu,
		#connect {
			margin: 0 auto;
		}
		#connect {
			margin-bottom: 20px;
		}
	}
</style>
