<script>
  import numeral from "numeral";
  import TreeNode from "./TreeNode.svelte";
  import Switch from "./Switch.svelte";
  import { prune } from "../helpers/jsondeep.js";
  export let data;
  let showChildren = true;
  let tree = [];
  const create = "#3c003b";
  const success = "#00FF00";
  const fail = "#FF0000";
  const default_fill = "#050505";
  const log_stroke = "#3c003b";
  let total_used_gas;

  let nodesLen = 1;
  if (data.children) {
      nodesLen = data.children.length;
  }
  $: {
    data = [data];
    total_used_gas = data[0].value;
  }

  // function split64(text) {
  //   for (let i = 0; i < text.length; i++) {
  //     if (text[i])
  //   }
  // }

  let depth = 1;
  let maxDepth = 1;

  function perc2color(perc) {
    perc = 100 - perc;
  	var r, g, b = 0;
  	if(perc < 50) {
  		r = 255;
  		g = Math.round(5.1 * perc);
  	}
  	else {
  		g = 255;
  		r = Math.round(510 - 5.10 * perc);
  	}
  	var h = r * 0x10000 + g * 0x100 + b * 0x1;
  	return '#' + ('000000' + h.toString(16)).slice(-6);
  }

  function percentageToColor(percentage, maxHue = 120, minHue = 0) {
    percentage = 1 - percentage;
    const hue = percentage * (maxHue - minHue) + minHue;
    return `hsl(${hue}, 100%, 50%)`;
  }

  function decMax() {
    if (maxDepth > 0) {
      maxDepth -= 1;
    }
  }
  function incMax() {
    maxDepth += 1;
  }

  let revertOnly = true;
  let logsActive = true;
  let newContracts = true;
  let successActive = true;
  let reverts = true;


</script>
<div class="breaker"></div>
<div class="tree-header">
  <div class="maxDepth">
    <label class="mr5"> Force show revert: </label>
    <Switch bind:checked={revertOnly}></Switch>
  </div>
  <div class="maxDepth">
    <label class="mr5" for="maxd"> Max Depth:</label>
    <button on:click={decMax} class="db">-</button>
    <input class="quantity db" bind:value={maxDepth} id="maxd" default="1" type="number">
    <button on:click={incMax} class="db">+</button>
  </div>
  <!-- <div style="font-size:13px;"> -->
    <div class="legendHolder">
      <label>Logs: </label>
      <span on:click={() => logsActive = !logsActive} class="dot logged" class:logsActive></span>
    </div>
    <div class="legendHolder">
      <label>New Contract: </label>
      <span on:click={() => newContracts = !newContracts}  class="dot creation" class:newContracts ></span>
    </div>
    <div class="legendHolder">
      <label>Success: </label>
      <span on:click={() => successActive = !successActive} class="dot success" class:successActive ></span>
    </div>
    <div class="legendHolder">
      <label>Revert: </label>
      <span on:click={() => reverts = !reverts} class="dot failure" class:reverts ></span>
    </div>
  <!-- </div> -->
</div>
<div class="content">
  <ul class="tree">
    {#each data as _node, i}
    <TreeNode
      node={_node} index={i} let:node
      {logsActive}
      {newContracts}
      {successActive}
      {reverts}
      show={_node.log && logsActive || _node.success && successActive || _node.revert && reverts || _node.created && newContracts}
      isChild={true}
      isLast={i === data.length - 1}
      isFirst={i === 0}
      {depth} {maxDepth}
      parentSuccess={_node.success}
      {revertOnly}
      >
      <slot {node}>
        <div
            class="basediv"
            class:success={node.success && !node.created && !node.log}
            class:failure={node.revert && !node.log && !node.created}
            class:creation={node.created && !node.log}
            class:logged={node.log}
        >
          <p>
            <span style="color:{percentageToColor(node.value/total_used_gas)};">[{numeral(node.value).format('0,0')}]: </span>
            <span class="contract">{node.name + ":"}</span>
            <span class="function">{node.function}</span>
            {#if node.created}
              <span class="bytesize">{node.inputs} bytes</span>
            {/if}
          </p>
        </div>
        {#if node.inputs}
          <span class="input">({prune(node.inputs)})</span>
        {/if}

      </slot>
    </TreeNode>
    {/each}
  </ul>
</div>

<style>

  .logsActive {
    background: #0C172F;
  }

  .newContracts {
    background: #3c003b;
  }

  .successActive {
    background: #123112;
  }

  .reverts {
    background: #622020;
  }

  .mr5 {
    margin-right: 5px;
  }
  .breaker {
    width: 100%;
    height: 1px;
    background: #171717;
    margin: 15px 0px;
  }
  .tree-header {
    justify-content: space-around;
  }
  .legendHolder {
    font-size: 13px;
    align-items: center;
  }
  .maxDepth {
    font-size: 13px;
    padding: 0px;
    display: flex;
    flex-direction: row;
    justify-content: space-evenly;
    align-items: center;
  }

  .db {
    margin-left: 1px;
    padding: 0px 3px !important;
  }

  input[type="number"] {
    -webkit-appearance: textfield;
    -moz-appearance: textfield;
    appearance: textfield;
  }

  input[type=number]::-webkit-inner-spin-button,
  input[type=number]::-webkit-outer-spin-button {
    -webkit-appearance: none;
  }

  .number-input {
    border: 2px solid #ddd;
    display: inline-flex;
  }

  .number-input,
  .number-input * {
    box-sizing: border-box;
  }

  .function {
    color: #ff7efd;
  }
  .dot {
    height: 20px;
    width: 20px;
    border-radius: 50%;
    display: inline-block;
    margin: 0px 5px !important;
    cursor: pointer;
  }
  input {
    width: 50px;
  }
  .logged {
    border: 2px #0C172F solid !important;
    margin: 0px 1px;
  }

  .logged p {
    white-space: break-spaces;
  }

  div.logged:hover {
    background: #0C172F !important;
  }

  div.creation:hover {
    background: #3c003b;
  }

  .creation {
    border: 2px #3c003b solid !important;
  }

  .creation::before, .creation::after {
    border-bottom: 1px #3c003b solid !important;
  }

  .success {
    border: 2px #123112 solid !important;
  }

  div.success:hover {
    background: #123112;
  }

  .failure {
    border: 2px #622020 solid !important;
  }

  div.failure:hover {
    background: #622020;
  }

  .input {
    font-size: 13px;
  }

  .gas {
    color: #ccc;
  }
  .content {
      width:500px;
      /* margin:auto; */
      text-align: left;
      color: #FFF !important;
  }
  div {
    /* overflow: scroll !important; */
    /* overflow: hidden; */
    display: inline-flex;
    flex-direction: row;
    flex-wrap: wrap;
  }

  p {
    margin: 3px 0px;
  }

  .basediv {
    border-radius: 5px;
    padding:2px 5px;
    cursor: pointer;
    white-space: nowrap;
    font-size: 13px;
    border: 1px #ccc solid;
    /* overflow: hidden; */
  }

  /* .tree li div:hover {
    background: #ccc; color: #000; border: 1px solid #000;
  } */

  /* .tree li div:hover, .tree p:hover+ul li div p,
  .tree li div:focus, .tree p:focus+ul li div p {
    background: #ccc;
    color: #000 !important;
    border: 1px solid #000;
  } */

  /* .isChild div:hover, .isChild a:hover+ul li div p,
  .isChild div:focus, .isChild a:focus+ul li div p {
    background: #ccc; color: #000; border: 1px solid #000;
  } */

</style>
