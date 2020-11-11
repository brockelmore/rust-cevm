<script>
	export let name;
	import Stackviz from "./components/Stackviz.svelte";
	import Flame from "./components/Flame.svelte";
	import {
		process_trace
	} from "./helpers/trace_process";
	import Select from './components/svelte-select/Select.js';

	let tx_hash;
	async function sim() {
		let e = await fetch('http://localhost:2347/sim', {
			method: 'post',
			body: JSON.stringify({
				"hash": tx_hash,
				"in_place": false,
				"options": ["trace", "no_commit"]
			})
		});
		let dat = await e.json();
		console.log("sim", dat);

		let trace = dat["Ok"]["Sim"][0]["trace"];
		hist[dat["Ok"]["Sim"][0]['hash']] = trace;
		testHistory.push({
			'label': dat["Ok"]["Sim"][0]['hash'],
			'value': dat["Ok"]["Sim"][0]['hash'],
			'data': trace
		});
		testHistory = testHistory;
		window.localStorage.setItem("tests", JSON.stringify(hist));
		data = process_trace(trace);
		console.log("data", data);
	}


	let container
	let data;
	let height;
	let testHistory = [];
	let selectedTest;

	let selectedSrc = window.localStorage.getItem("selected_src");
	let avail_srcs = [];
	let src_tests;
	let curr_test = {};

	let avail_tests = []; //JSON.parse(window.localStorage.getItem('avail_tests'));

	function srcs_from_tests() {
		avail_srcs = [];
		for (var property in avail_tests) {
			if (avail_tests.hasOwnProperty(property)) {
				let as_list = avail_tests[property]["src"].split("/");
				let test_src = as_list[as_list.length - 1];
				avail_srcs.push({value: test_src, label: test_src, full: avail_tests[property]["src"]});
			}
		}
		const uniq = new Set(avail_srcs.map(e => JSON.stringify(e)));

		avail_srcs = Array.from(uniq).map(e => JSON.parse(e));
		avail_srcs = avail_srcs.sort(function(a, b) {
	    var textA = a.value.toUpperCase();
	    var textB = b.value.toUpperCase();
	    return (textA < textB) ? -1 : (textA > textB) ? 1 : 0;
		});

		update_tests(selectedSrc);
	}


	let hist = JSON.parse(window.localStorage.getItem('tests'));
	let src_dir = window.localStorage.getItem("default_src");
	if (src_dir) {
		console.log(src_dir)
		tests().then(() => {
			srcs_from_tests();
		});
	} else {
		src_dir = "/home/brock/yamV3/contracts"
		compile().then(() => {
			tests().then(() => {
				srcs_from_tests();
			});
		});
	}



	if (!hist) {
		hist = {}
	}
	for (var property in hist) {
		if (hist.hasOwnProperty(property)) {
			testHistory.push({
				'label': property,
				'value': property,
				'data': hist[property]
			});
		}
	}

	let value = ""
	let selectedHistory = null;

	const options = [{
			kind: 'stack',
			component: Stackviz
		},
		{
			kind: 'flame',
			component: Flame
		}
	]
	let selected = options[0];


	async function load_compiled() {
		let e = await fetch('http://localhost:2347/load_compiled', {
			method: 'post',
			body: JSON.stringify({
				"output_dir": src_dir + "/out"
			})
		});
		let r = await e.json();
		await tests();
	}

	async function compile() {
		let e = await fetch('http://localhost:2347/compile', {
			method: 'post',
			body: JSON.stringify({
				"input_dir": src_dir,
				"output_dir": src_dir + "/out"
			})
		});
		window.localStorage.setItem("default_src", src_dir);
		let r = await e.json();
		await tests();
	}

	async function tests() {
		let e = await fetch('http://localhost:2347/tests', {
			method: 'get'
		});
		let r = await e.json()
		r = r["Ok"]["Tests"];
		avail_tests = [];
		for (var property in r) {
			if (r.hasOwnProperty(property)) {
				for (let j = 0; j < r[property].length; j++) {
					let full = {};
					full[property] = [r[property][j]];
					avail_tests.push({
						"test": r[property][j],
						"full": full,
						"src": property
					})
				}
			}
		}
		window.localStorage.setItem('avail_tests', JSON.stringify(avail_tests));
		avail_tests = avail_tests;
		await srcs_from_tests();
	}

	async function test() {
		curr_test = {}
		for (let i = 0; i < avail_tests.length; i++) {
			if (avail_tests[i].src == selectedSrc) {
				if (avail_tests[i].test == selectedTest) {
					curr_test["full"] = avail_tests[i]["full"]
					curr_test["src"] = avail_tests[i]["src"]
					curr_test["test"] = avail_tests[i]["test"]
				}
			}
		}
		if (!curr_test["full"]) {
				console.log("Couldn't find test");
				return
		}
		let e = await fetch('http://localhost:2347/test', {
			method: 'post',
			body: JSON.stringify({
				'tests': curr_test["full"],
				"options": {
					"testerIsEOA": true
				}
			})
		});
		let dat = await e.json();
		let num_traces = dat[curr_test["src"]][curr_test["test"]]["Test"].length;
		for (let j = 0; j < num_traces; j++) {
			let trace = dat[curr_test["src"]][curr_test["test"]]["Test"][j]
			hist[curr_test["test"] + ":" + j.toString()] = trace;
			testHistory.push({
				'label': curr_test["test"] + j.toString(),
				'value': curr_test["test"] + j.toString(),
				'data': trace
			});
		}
		testHistory = testHistory;
		window.localStorage.setItem("tests", JSON.stringify(hist));
		data = process_trace(
			dat[curr_test["src"]][curr_test["test"]]["Test"][num_traces - 1]["trace"]);
		console.log("data", data);
	}


	function loadHist(selected) {
		console.log(selected.detail.label, testHistory)
		for (let i = 0; i < testHistory.length; i++) {
			if (testHistory[i].label == selected.detail.label) {
				data = process_trace(testHistory[i]["data"]["trace"]);
			}
		}
		data = data;
	}



	function update_tests(selected_val) {
		if (selected_val) {
			if (selected_val.detail) {
				selectedSrc = selected_val.detail.full;
			} else {
				selectedSrc = selected_val
			}
		} else {
			return;
		}

		src_tests = [];
		for (var property in avail_tests) {
			if (avail_tests.hasOwnProperty(property)) {
				if (avail_tests[property]["src"] == selectedSrc) {
					src_tests.push({value: avail_tests[property]["test"], label: avail_tests[property]["test"], src: selectedSrc});
				}
			}
		}
		src_tests = src_tests.sort(function(a, b) {
	    var textA = a.value.toUpperCase();
	    var textB = b.value.toUpperCase();
	    return (textA < textB) ? -1 : (textA > textB) ? 1 : 0;
		})
	}

	function update_sel_test(selected_t) {
		selectedTest = selected_t.detail.value;
	}

	$: {
	}
</script>

<main>
	<div class='options'>
		<div>
			<span class='selector-label'>Source Directory:</span>
			<input bind:value={src_dir}>
			<div>
				<button on:click={compile}>Compile</button>
				<button on:click={load_compiled}>Load Compiled</button>
			</div>
		</div>

		<div>
			<span class='selector-label'>Sim Mainnet Tx:</span>
			<input bind:value={tx_hash}>
			<div>
				<button on:click={sim}>Sim</button>
			</div>
		</div>

		<!-- <select bind:value={selected}>
			{#each options as option}
				<option value={option}>{option.kind}</option>
			{/each}
		</select>

		<select bind:value={selectedHistory} on:change={loadHist}>
				<option value=null selected disabled>pick</option>
			{#each testHistory as option (option.test)}
				<option value="{option.test}">{option.test}</option>
			{/each}
		</select> -->

		<div style='display: grid; grid-area: lm' class="options-element">
			<span class='selector-label'>Contract: </span>
			<div class="themed">
				<Select items={avail_srcs} on:select={update_tests}> </Select>
			</div>
		</div>

		<div style='display: grid; grid-area: m' class="options-element">
			<span class='selector-label'>Test: </span>
			<div class="themed">
				<Select items={src_tests} on:select={update_sel_test}> </Select>
			</div>
		</div>

		<div style='display: grid; grid-area: rm' class="options-element">
			<span class='selector-label'>Traces: </span>
			<div class="themed">
				<Select items={testHistory} on:select={loadHist}> </Select>
			</div>
		</div>

		<div style='display: grid; grid-area: r'>
			<button on:click={test}>Test</button>
		</div>
		<!-- <select bind:value={selectedTest}>
				<option value=null selected disabled>pick</option>
			{#each avail_tests as option (option.test)}
				<option value="{option.test}">{option.test}</option>
			{/each}
		</select> -->
	</div>

	<svelte:component this={selected.component} {data} />
</main>

<style>
	main {
		text-align: center;
		padding: 1em;
		max-width: 240px;
		margin: 0 auto;
	}

	.options-element {

		align-content: center;
		justify-content: center;
	}

	.selector-label {
		color: #FFF
	}

	.options {
		width: 100%;
		display: grid;
		grid-template-rows: 100fr;
		grid-template-columns: 20fr 20fr 20fr 20fr;
		grid-template-areas: "l lm m rm r";
		align-content: center;
		align-items: center;
		text-align: center;
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

	.themed {
		/* --font-family: "Roboto Mono", monospace; */
		--border: 0px solid #535353;
		--background: #050505;
		--listBackground: #050505;
		--itemHoverBG: #4C4C55;
		--borderFocusColor: #535353;
		--color: #ffffff;
		--width: 100px;
		--height: 99%;
		--font-size: 15px;
		--appearance: none;
		-webkit-appearance: none;
		-moz-appearance: none;
		--cursor: pointer;
		--position: relative;
		--inputPadding: 1px 0px 1px 0px !important;
		--borderRadius: 0px;
		--listBorderRadius: 0px;
		--listEmptyPadding: 5px 0px 5px 0px;
		/* --listTop: 8px; */
		/* --listLeft: -1px; */
		margin-bottom: 0px;
		width: 155px;
		height: 25px;
		border: 1px solid #535353;
		--inputFontSize: 14px;
	}
</style>
