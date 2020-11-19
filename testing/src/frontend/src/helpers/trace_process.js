import BigNumber from 'bignumber.js'

BigNumber.config({
  EXPONENTIAL_AT: 1000,
  DECIMAL_PLACES: 80,
});

const create = "#3c003b";
const success = "#00FF00";
const fail = "#FF0000";
const default_fill = "#050505";
const log_stroke = "#3c003b";

export function process_subtrace(x, num, depth, parent_num) {
    let me = {}
    me['id'] = depth + "-" + parent_num + "-" + num;
    me['name'] = x["name"];
    me['function'] = x['function'];
    if (x['name'] == '') {
      me["name"] = x["address"]; //.slice(0,5)+".." + x["address"].slice(x["address"].length - 3,);
    }
    me['value'] = x['cost'];
    me['stroke'] =  x['success'] ? success : fail;
    me['fill'] = x['created'] ? create : default_fill
    me['created'] = x['created']
    me['success'] = x['success']
    me['revert'] = !x['success']

    if (!me["created"]) {
      if (x['inputs']["Tokens"] && x['inputs']["Tokens"].length > 0) {
        me['inputs'] = process_inputs(x['inputs']['Tokens'])
      } else if (x['inputs']["String"] && x['inputs']["String"].length > 0) {
        me['inputs'] = [x['inputs']['String']]
      }
    }


    if (x['inner'].length > 0) {
      me['children'] = []
    }
    for (let i = 0; i < x['inner'].length; i++) {
      me['children'].push(process_subtrace(x["inner"][i], i, depth + 1, me['id']))
    }

    if (!x["created"]) {
      if (!x['success'] && x['output']['Tokens'] && x['output']['Tokens'].length > 0) {
        if (!me['children']) {
          me['children'] = []
        }
        let error = {
          'id': me['id'] + '-error',
          'name': 'Revert',
          'function': 'Reason: "' +  x['output']['Tokens'][0]['String']+'"',
          'value': x['value'],
          'stroke': fail,
          'success': false,
          'revert': true,
          'fill': default_fill
        }
        me['children'].push(error);
      } else if (x['success'] && x['output']['Tokens'] && x['output']['Tokens'].length > 0) {
        if (!me['children']) {
          me['children'] = []
        }
        let output = {
          'id': me['id'] + '-output',
          'name': 'Output',
          'function': process_inputs(x['output']['Tokens']).join(', '),
          'value': 0,
          'stroke': success,
          'success': true,
          'fill': default_fill
        }
        me['children'].push(output);
      } else if (x['success'] && x['output']['String'] && x['output']['String'].length > 0) {
        if (!me['children']) {
          me['children'] = []
        }
        let output = {
          'id': me['id'] + '-output',
          'name': 'Output',
          'function': x['output']['String'].length > 100 ?  x['output']['String'].slice(0, 100) + ".." : x['output']['String'],
          'value': 0,
          'success': true,
          'stroke': success,
          'fill': default_fill
        }
        me['children'].push(output);
      }
    }

    // console.log(x["logs"])
    if (x["logs"] && x["logs"].length > 0) {
      if (!me['children']) {
        me['children'] = []
      }
      for (let i = 0; i < x['logs'].length; i++) {
        if (x['logs'][i]['log']["Parsed"]) {
          let log = parse_log(x['logs'][i]['log']["Parsed"]);
          let output = {
            'id': me['id'] + '-log-' + i.toString(),
            'name': x['logs'][i]["name"],
            'function': x['logs'][i]["event"] +"\n ( " + JSON.stringify(log, null, 4) + " )",
            'value': 0,
            'stroke': log_stroke,
            'success': false,
            'created': false,
            'fill': default_fill,
            'log': true
          }
          me['children'].push(output);
        } else {
          let output = {
            'id': me['id'] + '-log-' + i.toString(),
            'name': 'Log',
            'function': x['logs'][i]["name"] + "\n ( " + JSON.stringify(x['logs'][i]['log']['NotParsed'], null, 4) + " )",
            'value': 0,
            'stroke': log_stroke,
            'success': false,
            'created': false,
            'fill': default_fill,
            'log': true
          }
          me['children'].push(output);
        }
      }
    }
    return me
}

function parse_log(log) {
  let parsed = {}
  for (var property in log) {
    if (log.hasOwnProperty(property)) {
      if (log[property].isArray) {
        let parsed_array = []
        for (let i = 0; i < log[property].length; i++) {
          let e = match_token(log[property][i]);
          parsed_array.push(e)
        }
        parsed[property] = parsed_array;
      } else {
        let e;
        if (property == "key") {
          // console.log(log[property])
          try {
            e = hex2a(log[property]["FixedBytes"].slice(2,));
          } catch (f) {
            e = match_token(log[property]);
          }
        } else {
          e = match_token(log[property]);
        }
        parsed[property] = e
      }
    }
  }
  return parsed
}

function parse_test_logs(logs) {
  let parsed = []
  let sub_parsed = {}
  for (let i = 0; i < logs.length; i++) {
    let log = logs[i];
    if (log["log"]["Parsed"] && log["log"]["Parsed"]["key"]) {
      log["key"] = log["log"]["Parsed"]["key"];
    }
    if (log["log"]["Parsed"] && log["log"]["Parsed"]["val"]) {
      log["val"] = log["log"]["Parsed"]["val"];
    }

    switch (log["event"]) {
      case "logs":
        break;
      case "log_bytes32":
        sub_parsed["Parsed"] = true;
        parsed.push(sub_parsed);
        sub_parsed = {}
        sub_parsed['name'] = log['name'];
        sub_parsed['event'] = hex2a(log["log"]["Parsed"]['']["FixedBytes"].slice(2,));
        break;
      case "log_named":
        parsed.push(sub_parsed);
        sub_parsed['named'] = match_token(log["val"]);
        break;
      case "log_named_address":
        sub_parsed[hex2a(log["key"]["FixedBytes"].slice(2,)).replace(" ", "")] = match_token(log["val"]);
        break;
      case "log_named_bytes32":
        sub_parsed[hex2a(log["key"]["FixedBytes"].slice(2,)).replace(" ", "")] = hex2a(log["val"]);
        break;
      case "log_named_bool":
        sub_parsed[hex2a(log["key"]["FixedBytes"].slice(2,)).replace(" ", "")] = match_token(log["val"]);
        break;
      case "log_named_decimal_int":
        sub_parsed[hex2a(log["key"]["FixedBytes"].slice(2,)).replace(" ", "")] = match_token(log["val"]);
        break;
      case "log_named_decimal_uint":
        sub_parsed[hex2a(log["key"]["FixedBytes"].slice(2,)).replace(" ", "")] = match_token(log["val"]);
        break;
      case "log_named_int":
        sub_parsed[hex2a(log["key"]["FixedBytes"].slice(2,)).replace(" ", "")] = match_token(log["val"]);
        break;
      case "log_named_uint":
        sub_parsed[hex2a(log["key"]["FixedBytes"].slice(2,)).replace(" ", "")] = match_token(log["val"]);
        break;
      case "log_named_string":
        sub_parsed[hex2a(log["key"]["FixedBytes"].slice(2,)).replace(" ", "")] = hex2a(log["val"]);
        break;
      default:
        parsed.push(parse_log(log))
    }
  }
  return parsed.slice(1,)
}

function hex2a(hexx) {
    var hex = hexx.toString();//force conversion
    var str = '';
    for (var i = 0; (i < hex.length && hex.substr(i, 2) !== '00'); i += 2)
        str += String.fromCharCode(parseInt(hex.substr(i, 2), 16));
    return str;
}

function match_token(token) {
  let parsed = ""
  for (var property in token) {
    if (token.hasOwnProperty(property)) {
      let big_num_tmp;
      switch (property) {
        case 'Int':
          big_num_tmp = new BigNumber(token[property]);
          parsed = big_num_tmp.toString();
          break;
        case 'Uint':
          big_num_tmp = new BigNumber(token[property]);
          parsed = big_num_tmp.toString();
          break;
        default:
          parsed = token[property];
      }
    }
  }
  return parsed;
}

function process_inputs(inputs) {
  // console.log('inputs', inputs)
  let outputs = []
  for (let i = 0; i < inputs.length; i++) {
    for (var property in inputs[i]) {
      if (inputs[i].hasOwnProperty(property)) {
        // Do things here
        if (inputs[i][property].isArray) {
          outputs.concat(process_inputs(inputs[i][property]))
        } else {
          outputs.push(match_token(inputs[i]))
        }
      }
    }
  }
  return outputs;
}

export function process_trace(trace) {
  // console.log(trace)
  let fin = {}
  fin["id"] = "0";
  fin['name'] = trace[0]["name"];
  fin['function'] = trace[0]['function'];
  fin['success'] = trace[0]['success'];
  fin['revert'] = !trace[0]['success'];
  fin['value'] = trace[0]['cost'];
  fin['created'] = trace[0]['created'];
  if (!fin["created"]) {
    if (trace[0]['inputs'].length > 0 ) {
      fin['inputs'] = process_inputs(trace[0]['inputs'])
    }
  }


  fin['children'] = []
  for (let i = 0; i < trace[0]['inner'].length; i++) {
    fin['children'].push(process_subtrace(trace[0]["inner"][i], i, 1, fin['id']))
  }
  if (!trace[0]['success'] && trace[0]['output']['Tokens'] && trace[0]['output']['Tokens'].length > 0) {
    if (!fin['children']) {
      fin['children'] = []
    }
    let error = {
      'id': fin['id'] + '-error',
      'name': 'Revert',
      'function': 'Reason: "' +  trace[0]['output']['Tokens'][0]['String']+'"',
      'value': trace[0]['value'],
      'stroke': fail,
      'fill': default_fill
    }
    fin['children'].push(error);
  } else if (trace[0]['success'] && trace[0]['output']['Tokens'] && trace[0]['output']['Tokens'].length > 0) {
    if (!fin['children']) {
      fin['children'] = []
    }
    let output = {
      'id': fin['id'] + '-output',
      'name': 'Output',
      'function': process_inputs(trace[0]['output']['Tokens']).join(', '),
      'value': 0,
      'stroke': success,
      'success': true,
      'fill': default_fill
    }
    fin['children'].push(output);
  }  else if (trace[0]['success'] && trace[0]['output']['String'] && trace[0]['output']['String'].length > 0) {
    if (!fin['children']) {
      fin['children'] = []
    }
    let output = {
      'id': fin['id'] + '-output',
      'name': 'Output',
      'function': trace[0]['output']['String'],
      'value': 0,
      'stroke': success,
      'success': true,
      'fill': default_fill
    }
    fin['children'].push(output);
  }

  if (trace[0]["logs"] && trace[0]["logs"].length > 0) {
    if (!fin['children']) {
      fin['children'] = []
    }


    let logs = parse_test_logs(trace[0]['logs']);

    for (let i = 0; i < logs.length; i++) {
      if (logs[i]["Parsed"]) {
        let output = {
          'id': fin['id'] + '-log-' + i.toString(),
          'name': logs[i]["name"],
          'function': logs[i]["event"] +"\n ( " + JSON.stringify(logs[i], null, 4) + " )",
          'value': 0,
          'stroke': log_stroke,
          'success': false,
          'created': false,
          'revert': false,
          'fill': default_fill,
          'log': true
        }
        fin['children'].push(output);
      } else {
        let output = {
          'id': fin['id'] + '-log-' + i.toString(),
          'name': 'Log',
          'function': logs[i]["name"] + "\n ( " + JSON.stringify(logs[i], null, 4) + " )",
          'value': 0,
          'stroke': log_stroke,
          'revert': false,
          'success': false,
          'created': false,
          'fill': default_fill,
          'log': true
        }
        fin['children'].push(output);
      }
    }
  }
  return fin
}
