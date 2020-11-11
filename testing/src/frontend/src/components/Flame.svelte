<script>
  import * as d3 from 'd3';
  import { flamegraph, defaultFlamegraphTooltip } from 'd3-flame-graph';
  export let data;

  let flame;
  let chart;
  let details;
  let width;
  let height;
  let tip;

  $: {
		if (flame && data) {
      // let tmp_d = {'children': data}
			width = flame.scrollWidth ;
			height = flame.scrollHeight || 1400;
		  // graph = flamegraph().width(width);
      // console.log(data);
      // d3.select("#flame")
      //   .datum(data)
      //   .call(graph);
      chart = flamegraph()
        .width(960)
        .minHeight(960)
        .cellHeight(50)
        //Example to sort in reverse order
        //.sort(function(a,b){ return d3.descending(a.name, b.name);})
        .title("")
        .onClick(onClick);
      console.log(chart);
      // Example on how to use custom a tooltip.
      tip = defaultFlamegraphTooltip()
        .html(function(d) { return "name: " + d.data.name + ", value: " + d.data.value; });
      chart.tooltip(tip);

      details = document.getElementById("details");
      chart.setDetailsElement(details);

      d3.select("#flame")
         .datum(data)
         .call(chart);

      function find(id) {
       var elem = chart.findById(id);
       if (elem){
         console.log(elem)
         chart.zoomTo(elem);
       }
      }

      function resetZoom() {
       chart.resetZoom();
      }

      function onClick(d) {
       console.info(`Clicked on ${d.data.name}, id: "${d.id}"`);
       history.pushState({ id: d.id }, d.data.name, `#${d.id}`);
      }
    }
	}
</script>


<div id="flame" bind:this={flame}> </div>
<div id="details">
</div>
<style>
  #flame {
    width: 100%;
    height: 960px;
  }
</style>
