import json
import sys

def parse_criterion_output(input_file):
    results = {}
    with open(input_file, 'r') as f:
        for line in f:
            try:
                data = json.loads(line)
                if data['reason'] == 'benchmark-complete':
                    benchmark_id = data['id']
                    median_time = data['median']['estimate']
                    original_unit = data['median']['unit']

                    # Convert to seconds
                    if original_unit == 'ns':
                        median_time_s = median_time * 1e-9
                    elif original_unit == 'us':
                        median_time_s = median_time * 1e-6
                    elif original_unit == 'ms':
                        median_time_s = median_time * 1e-3
                    else:  # Assume it's already in seconds
                        median_time_s = median_time

                    results[benchmark_id] = {
                        "median_time": median_time_s
                    }
            except json.JSONDecodeError:
                continue  # Skip lines that are not valid JSON
    return results

def print_results(results):
    print("Benchmark Results (Median Time):")
    print("--------------------------------")
    for benchmark, data in results.items():
        print(f"{benchmark}: {data['median_time']:.9f} {data['unit']}")

def output_json(results, output_file):
    with open(output_file, 'w') as f:
        json.dump(results, f, indent=2)

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python script.py <input_file> <output_json_file>")
        sys.exit(1)

    input_file = sys.argv[1]
    output_file = sys.argv[2]
    results = parse_criterion_output(input_file)
    output_json(results, output_file)
