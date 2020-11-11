import {process_trace} from "./trace_process";

// updates tests and selected test
export function update_tests(meta_testing, update) {
	if (update) {
		if (update.detail) {
			meta_testing.selected_src = update.detail.full;
		} else {
			meta_testing.selected_src = update
		}
	} else {
		return meta_testing;
	}

	meta_testing.src_tests = [];
	for (var property in meta_testing.avail_tests) {
		if (meta_testing.avail_tests.hasOwnProperty(property)) {
			if (meta_testing.avail_tests[property]["src"] == meta_testing.selected_src) {
				meta_testing.src_tests.push({
					value: meta_testing.avail_tests[property]["test"],
					label: meta_testing.avail_tests[property]["test"],
					src: meta_testing.selected_src
				});
			}
		}
	}

	meta_testing.src_tests = meta_testing.src_tests.sort(function(a, b) {
		let textA = a.value.toUpperCase();
		let textB = b.value.toUpperCase();
		return (textA < textB) ? -1 : (textA > textB) ? 1 : 0;
	});

	return meta_testing;
}

// loads a trace into data
export function load_history(meta_testing, selected) {
	for (let i = 0; i < meta_testing.test_history.length; i++) {
		if (meta_testing.test_history[i].label == selected.detail.label) {
			meta_testing.data = process_trace(meta_testing.test_history[i]["data"]["trace"]);
		}
	}
	return meta_testing;
}

export async function test(meta_testing) {
	console.log("test", meta_testing);
	meta_testing.curr_test = {};
	// update curr test
	for (let i = 0; i < meta_testing.avail_tests.length; i++) {
		if (meta_testing.avail_tests[i].src == meta_testing.selected_src) {
			if (meta_testing.avail_tests[i].test == meta_testing.selected_test) {
				meta_testing.curr_test["full"] = meta_testing.avail_tests[i]["full"]
				meta_testing.curr_test["src"] = meta_testing.avail_tests[i]["src"]
				meta_testing.curr_test["test"] = meta_testing.avail_tests[i]["test"]
			}
		}
	}

	// log if it messed up
	if (!meta_testing.curr_test["full"]) {
		console.log("Couldn't find test");
		return
	}

	// execute test
	let e = await fetch('http://localhost:2347/test', {
		method: 'post',
		body: JSON.stringify({
			'tests': meta_testing.curr_test["full"],
			"options": {
				"testerIsEOA": true
			}
		})
	});

	// get trace info
	let dat = await e.json();
	let num_traces = dat[meta_testing.curr_test["src"]][meta_testing.curr_test["test"]]["Test"].length;
	for (let j = 0; j < num_traces; j++) {
		let trace = dat[meta_testing.curr_test["src"]][meta_testing.curr_test["test"]]["Test"][j]
		meta_testing.trace_history[meta_testing.curr_test["test"] + ":" + j.toString()] = trace;
		meta_testing.test_history.push({
			'label': meta_testing.curr_test["test"] + j.toString(),
			'value': meta_testing.curr_test["test"] + j.toString(),
			'data': trace
		});
	}

	// update storage
	window.localStorage.setItem("tests", JSON.stringify(meta_testing.trace_history));

	// parse last trace
	meta_testing.data = process_trace(
		dat[meta_testing.curr_test["src"]][meta_testing.curr_test["test"]]["Test"][num_traces - 1]["trace"]);

	return meta_testing;
}


export function srcs_from_tests(meta_testing) {
	meta_testing.avail_srcs = [];
	for (var property in meta_testing.avail_tests) {
		if (meta_testing.avail_tests.hasOwnProperty(property)) {
			let as_list = meta_testing.avail_tests[property]["src"].split("/");
			let test_src = as_list[as_list.length - 1];
			meta_testing.avail_srcs.push({
				value: test_src,
				label: test_src,
				full: meta_testing.avail_tests[property]["src"]
			});
		}
	}
	const uniq = new Set(meta_testing.avail_srcs.map(e => JSON.stringify(e)));

	meta_testing.avail_srcs = Array.from(uniq).map(e => JSON.parse(e));
	meta_testing.avail_srcs = meta_testing.avail_srcs.sort(function(a, b) {
		var textA = a.value.toUpperCase();
		var textB = b.value.toUpperCase();
		return (textA < textB) ? -1 : (textA > textB) ? 1 : 0;
	});

	meta_testing = update_tests(meta_testing, meta_testing.selected_src);
	return meta_testing;
}

export async function tests(meta_testing) {
	let e = await fetch('http://localhost:2347/tests', {
		method: 'get'
	});
	let r = await e.json()
	r = r["Ok"]["Tests"];
	meta_testing.avail_tests = [];
	for (var property in r) {
		if (r.hasOwnProperty(property)) {
			for (let j = 0; j < r[property].length; j++) {
				let full = {};
				full[property] = [r[property][j]];
				meta_testing.avail_tests.push({
					"test": r[property][j],
					"full": full,
					"src": property
				})
			}
		}
	}
	window.localStorage.setItem('avail_tests', JSON.stringify(meta_testing.avail_tests));
	return await srcs_from_tests(meta_testing);
}

export async function compile(meta_testing) {
	let e = await fetch('http://localhost:2347/compile', {
		method: 'post',
		body: JSON.stringify({
			"input_dir": meta_testing.src_dir,
			"output_dir": meta_testing.src_dir + "/out"
		})
	});
	window.localStorage.setItem("default_src", meta_testing.src_dir);
	let r = await e.json();
	return await tests(meta_testing);
}

export async function load_compiled(meta_testing) {
	console.log(meta_testing);
	let e = await fetch('http://localhost:2347/load_compiled', {
		method: 'post',
		body: JSON.stringify({
			"output_dir": meta_testing.src_dir + "/out"
		})
	});
	let r = await e.json();
	return await tests(meta_testing);
}


export async function sim(tx_hash, meta_testing) {
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
	meta_testing.trace_history[dat["Ok"]["Sim"][0]['hash']] = trace;
	meta_testing.test_history.push({
		'label': dat["Ok"]["Sim"][0]['hash'],
		'value': dat["Ok"]["Sim"][0]['hash'],
		'data': trace
	});
	window.localStorage.setItem("tests", JSON.stringify(meta_testing.trace_history));
	meta_testing.data = process_trace(trace);
	return meta_testing;
}
