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
    me['name'] = !x['created'] ? x["name"] + "::" + x['function'] : x["name"] ;
    if (x['name'] == '') {
      me["name"] = x["address"].slice(0,5)+".." + x["address"].slice(x["address"].length - 3,) + "::" + x['function'];
    }
    me['value'] = x['cost'];
    me['stroke'] =  x['success'] ? success : fail;
    me['fill'] = x['created'] ? create : default_fill
    me['created'] = x['created']

    if (x['inputs']["Tokens"] && x['inputs']["Tokens"].length > 0) {
      me['inputs'] = process_inputs(x['inputs']['Tokens'])
    } else if (x['inputs']["String"] && x['inputs']["String"].length > 0) {
      me['inputs'] = [x['inputs']['String']]
    }

    if (x['inner'].length > 0) {
      me['children'] = []
    }
    for (let i = 0; i < x['inner'].length; i++) {
      me['children'].push(process_subtrace(x["inner"][i], i, depth + 1, me['id']))
    }


    if (!x['success'] && x['output']['Tokens'] && x['output']['Tokens'].length > 0) {
      if (!me['children']) {
        me['children'] = []
      }
      let error = {
        'id': me['id'] + '-error',
        'name': 'Revert::Reason: "' +  x['output']['Tokens'][0]['String']+'"',
        'value': x['value'],
        'stroke': fail,
        'fill': default_fill
      }
      me['children'].push(error);
    } else if (x['success'] && x['output']['Tokens'] && x['output']['Tokens'].length > 0) {
      if (!me['children']) {
        me['children'] = []
      }
      let output = {
        'id': me['id'] + '-output',
        'name': 'Output: ' +  "( " + process_inputs(x['output']['Tokens']).join(', ') + " )",
        'value': 0,
        'stroke': success,
        'fill': default_fill
      }
      me['children'].push(output);
    } else if (x['success'] && x['output']['String'] && x['output']['String'].length > 0) {
      if (!me['children']) {
        me['children'] = []
      }
      let output = {
        'id': me['id'] + '-output',
        'name': 'Output: ' +  "( " + x['output']['String'].length > 100 ?  x['output']['String'].slice(0, 100) + ".." : x['output']['String'] + " )",
        'value': 0,
        'stroke': success,
        'fill': default_fill
      }
      me['children'].push(output);
    }

    console.log(x["logs"])
    // if (x["logs"] && x["logs"].length > 0) {
    //   if (!me['children']) {
    //     me['children'] = []
    //   }
    //   for (let i = 0; i < x['logs'].length; i++) {
    //     if (x['logs'][i]['log']["Parsed"]) {
    //       let output = {
    //         'id': me['id'] + '-log-' + i.toString(),
    //         'name': 'Log::' + x['logs'][i]["name"] + "\n ( " + JSON.stringify(x['logs'][i]['log']['Parsed']) + " )",
    //         'value': 0,
    //         'stroke': log_stroke,
    //         'fill': default_fill
    //       }
    //       me['children'].push(output);
    //     } else {
    //       let output = {
    //         'id': me['id'] + '-log-' + i.toString(),
    //         'name': 'Log::' + x['logs'][i]["name"] + "\n ( " + JSON.stringify(x['logs'][i]['log']['NotParsed']) + " )",
    //         'value': 0,
    //         'stroke': log_stroke,
    //         'fill': default_fill
    //       }
    //       me['children'].push(output);
    //     }
    //   }
    // }
    return me
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
  console.log(trace)
  let fin = {}
  fin["id"] = "0";
  fin['name'] = trace[0]["name"] + "::" + trace[0]['function'];
  fin['success'] = trace[0]['success'];
  fin['value'] = trace[0]['cost'];
  fin['created'] = trace[0]['created'];
  if (trace[0]['inputs'].length > 0 ) {
    fin['inputs'] = process_inputs(trace[0]['inputs'])
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
      'name': 'Revert::Reason: "' +  trace[0]['output']['Tokens'][0]['String']+'"',
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
      'name': 'Output: ' +  "( " + process_inputs(trace[0]['output']['Tokens']).join(', ') + " )",
      'value': 0,
      'stroke': success,
      'fill': default_fill
    }
    fin['children'].push(output);
  }  else if (trace[0]['success'] && trace[0]['output']['String'] && trace[0]['output']['String'].length > 0) {
    if (!fin['children']) {
      fin['children'] = []
    }
    let output = {
      'id': fin['id'] + '-output',
      'name': 'Output: ' +  "( " + trace[0]['output']['String'] + " )",
      'value': 0,
      'stroke': success,
      'fill': default_fill
    }
    fin['children'].push(output);
  }
  return fin
}
