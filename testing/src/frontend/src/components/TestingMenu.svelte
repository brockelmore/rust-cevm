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

<div class="menu">
  <span class='src selector-label'>Source Directory:</span>
  <input class='srcsel' bind:value={meta_testing.src_dir}>
  <div class='srcexec'>
    <button on:click={() => compile(meta_testing).then(() => { update(); }) }>Compile</button>
    <button on:click={() => load_compiled(meta_testing).then(() => { update(); }) }>Load Compiled</button>
  </div>

  <span class='c'>Contract: </span>
  <div class="csel themed">
    <Select items={meta_testing.avail_srcs} on:select={(e) => {update_tests(meta_testing, e); update(); }}> </Select>
  </div>

  <span class='t selector-label'>Test: </span>
  <div class="tsel themed">
    <Select items={meta_testing.src_tests} on:select={(e) => { meta_testing.selected_test = e.detail.value; update(); }}> </Select>
  </div>
  <button class='texec' on:click={() => {test(meta_testing).then(() => { update()}); }}>Test</button>

	<span class='tr selector-label'>Traces: </span>
	<div class="trsel themed">
		<Select items={meta_testing.test_history} on:select={(e) => {load_history(meta_testing, e); update(); }}> </Select>
	</div>


</div>


<style>
  .menu {
    display: grid;
    grid-template-columns: 115px 50fr 40fr;
    grid-template-rows: 25fr 25fr 25fr 25fr;
    grid-template-areas:
          "src srcsel srcexec"
          "c csel cexec"
          "t tsel texec"
          "tr trsel trexec";
    font-size: 13px;
    max-width: 700px;
    align-items: center;
    margin: 5px 0px;
  }

  .src {
    display: grid;
    grid-area: src;
    text-align: left;
  }

  .srcsel {
    display: grid;
    grid-area: srcsel;
    border: 1px solid #ccc;
    background: #000;
    color: #fff;
    border-radius: 0px;
    /* width: auto; */
    width: 298px;
  }

  .srcexec {
    display: grid;
    grid-area: srcexec;
    grid-template-rows: 100fr;
    grid-template-columns: 50fr 50fr;
  }

  .c {
    display: grid;
    grid-area: c;
    text-align: left;
  }

  .csel {
    display: grid;
    grid-area: csel;
  }

  .cexec {
    display: grid;
    grid-area: cexec;
  }

  .t {
    display: grid;
    grid-area: t;
    text-align: left;
  }

  .tsel {
    display: grid;
    grid-area: tsel;
  }

  .texec {
    display: grid;
    grid-area: texec;
  }

  .tr {
    display: grid;
    grid-area: tr;
    text-align: left;
  }

  .trsel {
    display: grid;
    grid-area: trsel;
  }

  .trexec {
    display: grid;
    grid-area: trexec;
  }



  .tester {
    left: 56px;
    position: relative;
  }
  .src-select {
    font-size: 13px;
    display: flex;
    justify-content: space-between;
    max-width: 500px;
    align-items: center;
    margin: 5px 0px;
  }

  .src-select input {
    border: 1px solid #ccc;
    background: #000;
    color: #fff;
    border-radius: 0px;
    width: auto;
  }

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
    width: 296px;
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
