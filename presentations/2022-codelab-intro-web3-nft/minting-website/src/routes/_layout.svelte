<script>
	import Nav from "../components/Nav.svelte";
	import Title from "../components/Title.svelte";
	import Footer from '../components/Footer.svelte';
</script>

<div>
	<Title />
	<Nav/>

	<main>
		<div>
			<slot />
		</div>
	</main>

	<Footer />
</div>

<style>
	div {
		width: 55%;
		margin: 0 auto;
	}

	@media screen and (max-width: 600px) {
		div {
			width: 95%;
		}
	}

	main {
		position: relative;
		padding-top: 2em;
		margin: 0 auto;
	}
</style>
