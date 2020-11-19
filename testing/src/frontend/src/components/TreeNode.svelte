<script>
  export let node;
  export let index;
  export let show;
  export let isChild;
  export let isLast;
  export let isFirst;
  export let depth;
  export let maxDepth;
  export let parentSuccess;
  export let revertOnly;
  export let logsActive;
  export let newContracts;
  export let successActive;
  export let reverts;

  let showChildren = true;
  $: {
    if (depth > maxDepth) {
      showChildren = false;
    } else {
      showChildren = true;
    }
  }


  let nextDepth = depth + 1;

  function changeArrow(e) {
    if (e.target.nodeName != "LI" && e.target.nodeName != "UL") {
      showChildren = !showChildren;
    }
    e.stopPropagation();
  }

  $:{console.log(node, newContracts)}
</script>

{#if show}
  <li on:click={changeArrow}
    class:isFirst
    class:isChild
    class:isLast
    class:success={node.success && !node.created && !node.log}
    class:failure={!node.success && !node.log}
    class:creation={node.created && !node.log}
    class:logged={node.log}
    class:parentSuccess
    class:parentFailure={!parentSuccess}
  >
    <slot {node} />
    {#if node.children}
      <div
        class:arrow-up={!showChildren}
        class:arrow-down={showChildren}
        class:arrow-up-success={!showChildren && node.success}
        class:arrow-down-success={showChildren && node.success}
        class:arrow-up-failure={!showChildren && node.revert}
        class:arrow-down-failure={showChildren && node.revert}
      >
      </div>
      <!-- show={showChildren || (!node.log && !node.success && revertOnly)} -->
      <ul>
      {#each node.children as _node, i}
        <svelte:self
          node={_node}
          index={i}
          let:node
          {logsActive}
          {newContracts}
          {successActive}
          {reverts}
          show={showChildren && (_node.log && logsActive || _node.success && successActive || _node.revert && reverts || _node.created && newContracts)}
          isChild={true}
          isLast={i === node.children.length - 1}
          isFirst={i === 0}
          depth={nextDepth} {maxDepth}
          parentSuccess={node.success}
          {revertOnly}
          >
          <slot {node} />
        </svelte:self>
      {/each}
      </ul>
    {/if}
  </li>
{/if}


<style>

  li {
    white-space: nowrap;
  }

  .parentSuccess::before, .parentSuccess::after {
    border-left: 1px #123112 solid !important;
  }

  .parentFailure::before, .parentFailure::after  {
    border-left: 1px #622020 solid !important;
  }

  .logged::before {
    border-bottom: 1px #0C172F solid !important;
  }
  .logged::after {
    border-top: 1px #0C172F solid !important;
  }

  .creation::before {
    border-bottom: 1px #3c003b solid !important;
  }
  .creation::after {
    border-top: 1px #3c003b solid !important;
  }

  .success::before{
    border-bottom: 1px #123112 solid !important;
  }
  .success::after {
    border-top: 1px #123112 solid !important;
  }

  .failure::before {
    border-bottom: 1px #622020 solid !important;
  }
  .failure::after {
    border-top: 1px #622020 solid !important;
  }

  .isChild {
    list-style-type: none;
    margin:10px;
    position: relative;
  }

  /* ul > li > a {
    background: red !important;
      border-radius: 0 0 0 5px !important;
  } */

  .isLast::after {
    display: none;
  }

  .isLast::before {
    border-radius: 0 0 0 5px !important;
  }

  .isChild::before {
    content: "";
    position: absolute;
    top:-18px;
    left:-20px;
    border-left: 1px solid #ccc;
    border-bottom:1px solid #ccc;
    /* border-radius:0 0 0 0px; */
    width:19px;
    height:33px;
  }

  .isChild::after {
    position:absolute;
    content:"";
    top:15px;
    left:-20px;
    border-left: 1px solid #ccc;
    border-top:1px solid #ccc;
    /* border-radius:0px 0 0 0; */
    width:20px;
    height:100%;
  }

  .arrow-up-success {
    border-bottom: 8px solid #123112;
  }

  .arrow-up-failure {
    border-bottom: 8px solid #622020;
  }

  .arrow-down-success {
    border-top: 8px solid #123112;
  }

  .arrow-down-failure {
    border-top: 8px solid #622020;
  }

  .arrow-up {
    position: relative;
    right: 14px;
    bottom: 22px;
    width: 0;
    height: 0;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    /* border-bottom: 8px solid #FFF; */
    z-index: 2;
  }

  .arrow-down {
    position: relative;
    right: 14px;
    bottom: 15px;
    width: 0;
    height: 0;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    /* border-top: 8px solid #FFF; */
    z-index: 2;
  }
</style>
