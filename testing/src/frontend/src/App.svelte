<script>
	import Tree from "./components/Tree.svelte";
	import TestingMenu from "./components/TestingMenu.svelte";
	import Flame from "./components/Flame.svelte";
	import {
		process_trace
	} from "./helpers/trace_process";
	import { update_tests, load_history, test, srcs_from_tests, tests, compile, load_compiled, sim } from "./helpers/testing";



	let tx_hash;
	let container;
	let height;

	let selected_src = window.localStorage.getItem("selected_src");
	let trace_history = JSON.parse(window.localStorage.getItem('tests'));
	let src_dir = window.localStorage.getItem("default_src");
	let test_history = [];
	if (!trace_history) {
		trace_history = {}
	}
	for (var property in trace_history) {
		if (trace_history.hasOwnProperty(property)) {
			test_history.push({
				'label': property,
				'value': property,
				'data': trace_history[property]
			});
		}
	}

	let meta_testing = {
		src_dir,
		selected_src,
		trace_history,
		test_history,
		selected_test: "",
		account: "",
		avail_srcs: [],
		avail_tests: [],
		src_tests: [],
		curr_test: {},
		data: []
	};

	if (meta_testing.src_dir) {
		tests(meta_testing).then(e => {
			meta_testing = e;
		});
	} else {
		console.log("no src_dir");
		meta_testing.src_dir = "/home/brock/yamV3/contracts"
		load_compiled(meta_testing).then(e => {
			meta_testing = e;
		});
	}

	let value = ""
	let selectedHistory = null;

	const options = [
		{
			kind: 'stack',
			component: Tree
		},
		{
			kind: 'flame',
			component: Flame
		}
	]
	let selected = options[0];



	function update_testing(event) {
		console.log("updating");
		meta_testing = meta_testing;
	}

	let data;
	$: data = meta_testing.data;

	$: {
		console.log("top level", meta_testing)
	}
</script>

<main>
	<div class='options'>

		<!-- <select bind:value={selectedTest}>
				<option value=null selected disabled>pick</option>
			{#each avail_tests as option (option.test)}
				<option value="{option.test}">{option.test}</option>
			{/each}
		</select> -->
	</div>
	<TestingMenu on:message={update_testing} {meta_testing}></TestingMenu>
	<svelte:component this={selected.component} {data} />
</main>

<style>
	main {
		text-align: center;
		padding: 1em;
		max-width: 240px;
		margin: 0 auto;
		display: flex;
		flex-direction: column;
	}

	h1 {
		color: #fff;
		text-transform: uppercase;
		font-size: 4em;
		font-weight: 100;
	}

	btn {
		color: #fff;
	}

	@media (min-width: 640px) {
		main {
			max-width: none;
		}
	}
</style>
