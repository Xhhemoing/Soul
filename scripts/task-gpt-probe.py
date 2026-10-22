"""Bounded GPT-6 synthetic comparison; usage delta is observed, not assumed pricing."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import time
import urllib.request
import urllib.error

spec = importlib.util.spec_from_file_location("probe", Path(__file__).with_name("task-model-probe.py"))
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


def main(*, input_loader=None, instruction_builder=None, max_output_tokens=768):
    # Alternative research protocols share the same bounded call and billing path.
    # CLI defaults and the original fixed-option protocol remain unchanged.
    if max_output_tokens not in (768, 3072):
        raise ValueError("unsupported_research_output_limit")
    input_loader = input_loader or probe.inputs
    instruction_builder = instruction_builder or probe.instructions
    p = argparse.ArgumentParser()
    p.add_argument("--inputs", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    p.add_argument("--model", choices=["gpt-6-luna", "gpt-6-sol", "gpt-6.1-sol", "gpt-6-astra"], required=True)
    p.add_argument("--offset", type=int, default=0)
    p.add_argument("--limit", type=int, default=1)
    p.add_argument("--ledger", type=Path)
    args = p.parse_args()
    key = os.environ["SOUL_PROBE_GPT_KEY"]
    opener = urllib.request.build_opener(probe.NoRedirect())
    def call(route, body=None):
        req = urllib.request.Request("https://api.xhh.cloudns.be/v1/"+route,
            json.dumps(body, ensure_ascii=False).encode() if body is not None else None,
            {"Authorization": "Bearer "+key, "Content-Type": "application/json"})
        with opener.open(req, timeout=90) as response:
            return json.load(response)
    def cost():
        # Do not save or print account history, balances, or unrelated usage.
        raw = call("usage")
        if raw.get("unit") != "USD":
            raise ValueError("unknown_billing_unit")
        return raw["usage"]["total"]["actual_cost"], raw["usage"]["total"]["requests"]
    args.output.mkdir(parents=True, exist_ok=True)
    rows = input_loader(args.inputs)[args.offset:args.offset+args.limit]
    if not rows:
        raise ValueError("empty_input_slice")
    prompt = instruction_builder(rows[0])
    if any(instruction_builder(row) != prompt for row in rows):
        raise ValueError("mixed_input_metadata_modes")
    probe.write_new(args.output/f"run-{args.offset}.json", {"model":args.model,"prompt":prompt,
        "max_output_tokens":max_output_tokens,"reasoning_effort":"low","limit":args.limit,"automatic_retries":0})
    for row in rows:
        previous=[]
        if args.ledger and args.ledger.exists():
            previous=[json.loads(s) for s in args.ledger.read_text(encoding="utf-8").splitlines() if s]
        if any(x.get("api_cost_usd") is None for x in previous):
            raise ValueError("unsettled_previous_request_stop")
        known=sum(x["api_cost_usd"] for x in previous)
        # Sequential experiment, with >=25 cents headroom per bounded call.
        # Stop on ambiguous billing; never extrapolate a free or fixed unit price.
        headroom=max(0.25,2*max([x["api_cost_usd"] for x in previous]+[0]))
        if args.limit>1 and (args.ledger is None or not previous):
            raise ValueError("batch_requires_observed_pilot_billing")
        if known+headroom>4.5:
            raise ValueError("experiment_budget_stop")
        cid=row["case_id"]
        probe.write_new(args.output/f"input-{cid}.json", row)
        before, count = cost()
        record={"case_id":cid,"model":args.model,"api_cost_usd":None}
        start=time.perf_counter()
        try:
            raw=call("responses", {"model":args.model,"instructions":prompt,
                "input":[{"role":"user","content":json.dumps(row,ensure_ascii=False)}],
                "max_output_tokens":max_output_tokens,"reasoning":{"effort":"low"},"store":False})
            record["latency_ms"]=(time.perf_counter()-start)*1000
            saved={k:raw.get(k) for k in ["model","status","output","usage","incomplete_details","error"]}
            probe.write_new(args.output/f"raw-{cid}.json",saved)
            text="".join(c.get("text","") for o in raw.get("output",[]) if o.get("type")=="message" for c in o.get("content",[]) if c.get("type")=="output_text")
            with (args.output/f"prediction-{cid}.json").open("x",encoding="utf-8") as f:f.write(text)
            record.update({"inference_returned":True,"returned_model":raw.get("model"),"usage":raw.get("usage"),"status":raw.get("status")})
        except Exception as e:
            reason=type(e).__name__+(":"+str(e.code) if isinstance(e,urllib.error.HTTPError) else "")
            probe.write_new(args.output/f"failure-{cid}.json",{"reason":reason})
            record.update({"inference_returned":False,"failure":reason,"latency_ms":(time.perf_counter()-start)*1000})
        try:
            after, end_count=cost()
            for _ in range(3):
                if end_count>count:break
                time.sleep(1)
                after,end_count=cost()
        except Exception as e:
            record.update({"billing_verified_for_single_request":False,"billing_error":type(e).__name__})
            if args.ledger:
                probe.append(args.ledger,{"model":args.model,"case_id":cid,"api_cost_usd":None,"reason":"billing_query_failed"})
            probe.append(args.output/"measurements.jsonl",record)
            raise ValueError("billing_query_failed_stop") from None
        record["observed_usage_requests_delta"]=end_count-count
        record["observed_account_cost_delta_usd"]=after-before
        record["billing_verified_for_single_request"]=end_count-count==1 and after>before
        if record["billing_verified_for_single_request"]:
            record["api_cost_usd"]=after-before
            if args.ledger:
                probe.append(args.ledger,{"model":args.model,"case_id":cid,"api_cost_usd":after-before})
        probe.append(args.output/"measurements.jsonl",record)
        print(json.dumps({k:record.get(k) for k in ["case_id","model","inference_returned","latency_ms","api_cost_usd","failure","billing_verified_for_single_request"]}),flush=True)
        if not record["billing_verified_for_single_request"]:
            if args.ledger:
                probe.append(args.ledger,{"model":args.model,"case_id":cid,"api_cost_usd":None,"reason":"billing_delta_ambiguous"})
            raise ValueError("unverified_billing_stop")


if __name__=="__main__":main()
