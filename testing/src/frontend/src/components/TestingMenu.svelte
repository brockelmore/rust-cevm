<script>
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();
	import Select from './svelte-select/Select.js';
  import { update_tests, load_history, test, srcs_from_tests, tests, compile, load_compiled, sim } from "../helpers/testing";
  export let meta_testing;

  function update() {
    console.log("update here");
    dispatch("message", {text: "update"});
  }

</script>

<div>
	<div>
		<span class='selector-label'>Source Directory:</span>
		<input bind:value={meta_testing.src_dir}>
		<div>
			<button on:click={() => compile(meta_testing).then(() => {dispatch("update", {})})}>Compile</button>
			<button on:click={() => load_compiled(meta_testing).then(() => {dispatch("update", {})})}>Load Compiled</button>
		</div>
	</div>

	<!-- <div>
		<span class='selector-label'>Sim Mainnet Tx:</span>
		<input bind:value={tx_hash}>
		<div>
			<button on:click={sim}>Sim</button>
		</div>
	</div> -->

	<div class="options-element">
		<span class='selector-label'>Contract: </span>
		<div class="themed">
			<Select items={meta_testing.avail_srcs} on:select={(e) => {update_tests(meta_testing, e); update(); }}> </Select>
		</div>
	</div>

	<div class="options-element">
		<span class='selector-label'>Test: </span>
		<div class="themed">
			<Select items={meta_testing.src_tests} on:select={(e) => { meta_testing.selected_test = e.detail.value; update(); }}> </Select>
		</div>
	</div>

	<div class="options-element">
		<span class='selector-label'>Traces: </span>
		<div class="themed">
			<Select items={meta_testing.test_history} on:select={(e) => {load_history(meta_testing, e); update(); }}> </Select>
		</div>
	</div>

	<div>
		<button on:click={() => test(meta_testing) }>Test</button>
	</div>
</div>


<style>
  .themed {
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
</style>
