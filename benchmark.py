import subprocess
import re
import numpy as np
import matplotlib.pyplot as plt
from tqdm import tqdm

def run_benchmark(min_res, max_res, step, d, nb_call_per_res):
    """
    Run benchmarks for different resolutions and plot the results.
    
    Args:
        min_res: Starting resolution
        max_res: Maximum resolution
        step: Step size for resolution increment
        d: Value for the -d parameter
        nb_call_per_res: Number of times to run each resolution for averaging
    """
    resolutions = list(range(min_res, max_res + 1, step))
    avg_times = []
    
    print(f"Running benchmarks from resolution {min_res} to {max_res} with step {step}")
    print(f"Each resolution will be tested {nb_call_per_res} times")
    
    for res in tqdm(resolutions):
        times = []
        
        for _ in range(nb_call_per_res):
            cmd = f"cargo run --release -- -v -p none -d {d} --reso {res} -j cpu"
            result = subprocess.run(cmd, shell=True, text=True, capture_output=True)
            
            # Extract the elapsed time from the output
            # Extract the elapsed time from the output - handle both ms and s formats
            ms_match = re.search(r"Time elapsed: ([\d.]+)ms", result.stdout)
            s_match = re.search(r"Time elapsed: ([\d.]+)s", result.stdout)
            
            if ms_match:
                time_ms = float(ms_match.group(1))
                times.append(time_ms)
            elif s_match:
                time_ms = float(s_match.group(1)) * 1000  # Convert seconds to milliseconds
                times.append(time_ms)
            else:
                time_ms = None
        
        if times:
            avg_time = sum(times) / len(times)
            avg_times.append(avg_time)
            print(f"Resolution {res}: Average time = {avg_time:.2f}ms")
        else:
            print(f"Resolution {res}: Failed to extract timing information")
            avg_times.append(0)
    
    # Plot the results
    plt.figure(figsize=(10, 6))
    plt.plot(resolutions, avg_times, marker='o')
    plt.title(f"Performance Benchmark JFA_wgpu")
    plt.xlabel("Resolution")
    plt.ylabel("Average Execution Time (ms)")
    plt.grid(True)
    
    # Save the plot and display it
    plt.savefig(f"benched_graphs/benchmark_d{d}.png")
    plt.show()
    
    # Save the data to a CSV file
    with open(f"benched_graphs/benchmark_d{d}.csv", "w") as f:
        f.write("Resolution,Average Time (ms)\n")
        for res, time in zip(resolutions, avg_times):
            f.write(f"{res},{time}\n")
    
    print(f"Results saved to benchmark_d{d}.png and benchmark_d{d}.csv")

if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description='Benchmark blue noise generator at different resolutions')
    parser.add_argument('--min_res', type=int, required=True, help='Minimum resolution')
    parser.add_argument('--max_res', type=int, required=True, help='Maximum resolution')
    parser.add_argument('--step', type=int, required=True, help='Resolution step size')
    parser.add_argument('--d', type=float, required=True, help='d parameter value')
    parser.add_argument('--nb_call_per_res', type=int, required=True, help='Number of runs per resolution')
    
    args = parser.parse_args()
    
    run_benchmark(
        args.min_res,
        args.max_res,
        args.step,
        args.d,
        args.nb_call_per_res
    )