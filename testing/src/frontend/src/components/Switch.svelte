<script>
  export let checked = false;
  export let color = "#2196F3";
</script>


<label class="switch">
  <input type="checkbox" bind:checked />
  <span class="slider" />
</label>

<style>
  .switch {
    position: relative;
    display: inline-block;
    width: 42px;
    height: 20px;
  }

  .switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: #ccc;
    -webkit-transition: 0.4s;
    transition: 0.4s;
    border-radius: 34px;
  }

  .slider:before {
    position: absolute;
    content: "";
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 1px;
    background-color: white;
    transition: 0.4s;
    border-radius: 50%;
  }

  input:checked + .slider {
    background-color: #622020;
  }

  input:checked + .slider {
    box-shadow: 0 0 1px #622020;
  }

  input:checked + .slider:before {
    -webkit-transform: translateX(20px);
    -ms-transform: translateX(20px);
    transform: translateX(20px);
  }
</style>
