<script>
  import G6 from '@antv/g6';
  export let data;
  let stackviz;
  let graph;
  let width;
  let height;
  let past_data;

  G6.registerNode('file-node', {
	  draw: function draw(cfg, group) {
	    const keyShape = group.addShape('rect', {
	      attrs: {
	        x: 10,
	        y: -12,
	        fill: cfg.fill || '#0d071a',
	        stroke: cfg.stroke || '#000',
	      },
	    });



	    let isLeaf = false;
	    if (cfg.collapsed) {
	      group.addShape('marker', {
	        attrs: {
	          symbol: 'triangle',
	          x: 4,
	          y: -2,
	          r: 4,
	          fill: '#666',
	        },
	        name: 'marker-shape-up',
	      });
	    } else if (cfg.children && cfg.children.length > 0) {
	      group.addShape('marker', {
	        attrs: {
	          symbol: 'triangle-down',
	          x: 4,
	          y: -2,
	          r: 4,
	          fill: '#666',
	        },
	        name: 'marker-shape-down',
	      });
	    } else {
	      isLeaf = true;
	    }
	    const shape = group.addShape('text', {
	      attrs: {
	        x: 20,
	        y: 7,
	        text: cfg.name,
	        fill: '#FFF',
	        fontSize: 16,
	        textAlign: 'bottom',
	      },
	      name: 'text-shape',
	    });



	    const bbox = shape.getBBox();

      const outputBox = group.addShape('rect', {
        attrs: {
          x: bbox.width + 60,
          y: 7,
          fill: '#FFF',
          stroke: '#FFF',
        },
      });
      if (cfg.inputs.length > 0) {
        let input = JSON.stringify(cfg.inputs);
        if (cfg.created) {
            input = input.length > 100 ? input.slice(0, 100) + "..]" : input
        }
        const outputShape = group.addShape('text', {
          attrs: {
            x: bbox.width + 60,
            y: 7,
            text: input, //.join(', ')),
            fill: '#FFF',
            fontSize: 16,
            textAlign: 'left',
          },
          name: 'text-shape',
        });
      }


	    let backRectW = bbox.width;
	    let backRectX = keyShape.attr('x');
	    if (!isLeaf) {
	      backRectW += 8;
	      backRectX -= 15;
	    }
	    keyShape.attr({
	      width: backRectW + 30,
	      height: bbox.height + 4,
	      x: backRectX,
	    });
	    return keyShape;
	  },
	});
	G6.registerEdge(
	  'step-line',
	  {
	    getControlPoints: function getControlPoints(cfg) {
	      const startPoint = cfg.startPoint;
	      const endPoint = cfg.endPoint;
	      return [
	        startPoint,
	        {
	          x: startPoint.x,
	          y: endPoint.y,
	        },
	        endPoint,
	      ];
	    },
	  },
	  'polyline',
	);


  $: {
		if (stackviz && data) {
      if (data != past_data) {
        console.log("new data");
          try {
            graph.clear()
          } catch (e) {
            console.log("couldnt destroy", e);
          }
      }
      width = stackviz.scrollWidth || 300;
      height = stackviz.scrollHeight || 1400;
      if (!past_data) {
        graph = new G6.TreeGraph({
          container: stackviz,
          width,
          height,
          maxZoom: 30,
          minZoom: .05,
          linkCenter: true,
          animateCfg: {
            duration: 150
          },
          modes: {
            default: [
              {
                type: 'collapse-expand',
                animate: false,
                onChange: function onChange(item, collapsed) {
                  const data = item.get('model');
                  data.collapsed = collapsed;
                  return true;
                },
              },
              'drag-canvas',
              'zoom-canvas',
            ],
          },
          defaultEdge: {
            style: {
              stroke: '#A3B1BF',
            },
          },
          layout: {
            type: 'indented',
            isHorizontal: true,
            direction: 'LR',
            indent: 180,
            getHeight: function getHeight() {
              return 32;
            },
            getWidth: function getWidth() {
              return 16;
            },
          },
        });
      }

			graph.node((node) => {
				return {
					type: 'file-node',
					label: node.name,
					fill: node.fill,
					stroke: node.stroke,
          inputs: node.inputs || [],
          created: node.created,
					collapsed: true,
				};
			});
			graph.edge(() => {
				return {
					type: 'step-line',
				};
			});

			graph.data(data);
			graph.render();
			graph.fitView();
      past_data = data;
		}
	}
</script>


<div id="stackviz" bind:this={stackviz}> </div>

<style>
  #stackviz {
    width: 100%;
  }
</style>
