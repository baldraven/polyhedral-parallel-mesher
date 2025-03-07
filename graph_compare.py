import argparse
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os
from matplotlib.ticker import FuncFormatter
import scipy.interpolate as interp

def extract_parameters_from_csv(file_path):
    """Extract benchmark parameters from the CSV file comments."""
    parameters = {}
    with open(file_path, 'r') as f:
        for line in f:
            if not line.startswith('#'):
                break
            if ':' in line:
                key, value = line.strip('# \n').split(':', 1)
                parameters[key.strip()] = value.strip()
    return parameters

def load_csv_data(file_path):
    """Load data from a benchmark CSV file."""
    # Read parameters from comments
    parameters = extract_parameters_from_csv(file_path)
    parameters['filename'] = file_path
    
    # Skip comment lines and read the actual data
    data = pd.read_csv(file_path, comment='#')
    
    return data, parameters

def generate_label(file_path):
    """Generate a label from the filename"""
    base = os.path.basename(file_path)
    return os.path.splitext(base)[0]

def interpolate_data(reference_x, reference_y, target_x):
    """Interpolate y values at target_x points based on reference data."""
    # Create interpolation function
    f = interp.interp1d(reference_x, reference_y, kind='linear', 
                         bounds_error=False, fill_value='extrapolate')
    
    # Interpolate y values at target_x points
    interpolated_y = f(target_x)
    
    return interpolated_y

def plot_speedup_comparison(base_csv, comparison_csv, output_file=None, plot_title=None, num_points=50):
    """
    Plot the speedup percentage of comparison_csv relative to base_csv using interpolation.
    
    Args:
        base_csv: Path to the baseline benchmark CSV
        comparison_csv: Path to the comparison benchmark CSV
        output_file: Path to save the output chart
        plot_title: Title for the plot
        num_points: Number of interpolated points to use for comparison
    """
    try:
        # Load data from both CSV files
        base_data, base_params = load_csv_data(base_csv)
        comp_data, comp_params = load_csv_data(comparison_csv)
        
        # Convert milliseconds to seconds
        base_data['Time (s)'] = base_data['Average Time (ms)'] / 1000
        comp_data['Time (s)'] = comp_data['Average Time (ms)'] / 1000
        
        # Create figure with better proportions
        fig, ax = plt.subplots(figsize=(14, 8))
        
        # Extract labels for the legend
        base_label = generate_label(base_csv)
        comp_label = generate_label(comparison_csv)
        
        # Determine the overlapping range of resolutions
        min_res = max(base_data['Resolution'].min(), comp_data['Resolution'].min())
        max_res = min(base_data['Resolution'].max(), comp_data['Resolution'].max())
        
        # Check if there's a valid overlapping range
        if (min_res > max_res):
            print("Error: No overlapping resolution range between the two datasets.")
            print(f"Base dataset range: {base_data['Resolution'].min()} to {base_data['Resolution'].max()}")
            print(f"Comparison dataset range: {comp_data['Resolution'].min()} to {comp_data['Resolution'].max()}")
            return
            
        print(f"Comparing over overlapping resolution range: {min_res} to {max_res}")
        
        # Generate evenly spaced resolutions for interpolation within the overlapping range
        interp_resolutions = np.linspace(min_res, max_res, num_points)
        
        # Filter datasets to include only points within the overlapping range
        base_data_filtered = base_data[
            (base_data['Resolution'] >= min_res) & 
            (base_data['Resolution'] <= max_res)
        ]
        
        comp_data_filtered = comp_data[
            (comp_data['Resolution'] >= min_res) & 
            (comp_data['Resolution'] <= max_res)
        ]
        
        # Check that we have enough data points for interpolation
        if len(base_data_filtered) < 2 or len(comp_data_filtered) < 2:
            print("Error: Not enough data points within the overlapping range for interpolation.")
            print(f"Base dataset: {len(base_data_filtered)} points")
            print(f"Comparison dataset: {len(comp_data_filtered)} points")
            return
        
        # Create interpolation functions for both datasets
        try:
            base_interp = interp.interp1d(
                base_data_filtered['Resolution'], 
                base_data_filtered['Time (s)'],
                kind='cubic' if len(base_data_filtered) >= 4 else 'linear',
                bounds_error=False
            )
        except ValueError as e:
            print(f"Error creating interpolation for base data: {e}")
            # Fall back to linear interpolation if cubic fails
            base_interp = interp.interp1d(
                base_data_filtered['Resolution'], 
                base_data_filtered['Time (s)'],
                kind='linear',
                bounds_error=False
            )
            
        try:
            comp_interp = interp.interp1d(
                comp_data_filtered['Resolution'], 
                comp_data_filtered['Time (s)'],
                kind='cubic' if len(comp_data_filtered) >= 4 else 'linear',
                bounds_error=False
            )
        except ValueError as e:
            print(f"Error creating interpolation for comparison data: {e}")
            # Fall back to linear interpolation if cubic fails
            comp_interp = interp.interp1d(
                comp_data_filtered['Resolution'], 
                comp_data_filtered['Time (s)'],
                kind='linear',
                bounds_error=False
            )
        
        # Calculate interpolated times for both datasets
        base_times = base_interp(interp_resolutions)
        comp_times = comp_interp(interp_resolutions)
        
        # Filter out invalid values (NaN, 0, infinity)
        valid_indices = []
        for i, (base_time, comp_time) in enumerate(zip(base_times, comp_times)):
            if (np.isfinite(base_time) and np.isfinite(comp_time) and 
                base_time > 0 and comp_time > 0):
                valid_indices.append(i)
        
        # If we lost some points, notify the user
        if len(valid_indices) < len(interp_resolutions):
            print(f"Warning: Filtered out {len(interp_resolutions) - len(valid_indices)} invalid data points")
            
        # Create new arrays with only valid data
        filtered_resolutions = [interp_resolutions[i] for i in valid_indices]
        filtered_base_times = [base_times[i] for i in valid_indices]
        filtered_comp_times = [comp_times[i] for i in valid_indices]
        
        # If no valid data points remain, exit
        if not filtered_resolutions:
            print("Error: No valid data points after filtering.")
            return
        
        # Update our working arrays
        interp_resolutions = np.array(filtered_resolutions)
        base_times = np.array(filtered_base_times)
        comp_times = np.array(filtered_comp_times)
        
        # Plot only the data points within the comparison range
        ax2 = ax.twinx()
        
        # Plot points within the comparison range
        ax2.plot(base_data_filtered['Resolution'], base_data_filtered['Time (s)'], 'o', color='darkblue', 
                 alpha=0.7, label=f"{base_label}")
        ax2.plot(comp_data_filtered['Resolution'], comp_data_filtered['Time (s)'], 's', color='darkred', 
                 alpha=0.7, label=f"{comp_label}")
        
        # Plot the interpolated curves
        ax2.plot(interp_resolutions, base_times, '-', color='darkblue', alpha=0.7)
        ax2.plot(interp_resolutions, comp_times, '-', color='darkred', alpha=0.7)
        
        ax2.set_ylabel("Temps d'exécution (secondes)", fontsize=12)
        ax2.legend(loc='upper left', fontsize=9)
        
        # Calculate speedup percentages using interpolated data
        speedups = []
        for base_time, comp_time in zip(base_times, comp_times):
            # Calculate speedup factor: base_time / comp_time
            # Values > 1 mean the comparison is faster (e.g., 2.0 = twice as fast)
            speedup = base_time / comp_time
            # Convert to percentage (e.g., 2.0 = 200%)
            speedup_percent = speedup * 100
            speedups.append(speedup_percent)
        
        # For the bar chart, use fewer bars to avoid crowding
        # Calculate a reasonable number of bars based on the range
        range_width = max(interp_resolutions) - min(interp_resolutions)
        bar_count = min(20, max(5, min(len(interp_resolutions), int(range_width / 200))))  # Adjust as needed
        
        # Ensure we don't try to create more bars than we have data points
        bar_indices = np.round(np.linspace(0, len(interp_resolutions)-1, bar_count)).astype(int)
        bar_resolutions = [interp_resolutions[i] for i in bar_indices]
        bar_speedups = [speedups[i] for i in bar_indices]
        
        # Plot the speedups as bars with color gradient
        bar_width = range_width / (len(bar_resolutions) * 1.3)
        bars = ax.bar(bar_resolutions, bar_speedups, alpha=0.7, width=bar_width)
        
        # Color the bars based on speedup value - values below 100% are red, above are green
        for bar, speedup in zip(bars, bar_speedups):
            if speedup > 100:
                bar.set_color('green')
                bar.set_alpha(min(0.3 + (speedup - 100) / 200, 0.9))  # More intense green for higher speedup
            else:
                bar.set_color('red')
                bar.set_alpha(min(0.3 + (100 - speedup) / 100, 0.9))  # More intense red for lower speedup
        
        # Add a horizontal line at y=100% (no speedup) for reference
        ax.axhline(y=100, color='black', linestyle='--', alpha=0.5, zorder=0)
        
        # Add value labels on top of each bar
        max_speedup = max(speedups)
        for bar, speedup in zip(bars, bar_speedups):
            if speedup > 100:
                # For speedups > 100%, show as "×N faster"
                multiplier = speedup / 100
                text = f'×{multiplier:.1f}'
                va = 'bottom'
                offset = max_speedup * 0.02
            else:
                # For speedups <= 100%, show as "N%"
                text = f'{speedup:.0f}%'
                va = 'bottom' if speedup >= 50 else 'top'
                offset = 0 if speedup >= 50 else -max_speedup * 0.02
                
            ax.text(
                bar.get_x() + bar.get_width()/2.,
                speedup + offset,
                text,
                ha='center', va=va,
                color='black', fontweight='bold', fontsize=9
            )
        
        
        # Set chart title and labels
        if plot_title:
            plt.title(f"{plot_title}", fontsize=16)
        else:
            plt.title(f"Comparaison de performance: {comp_label} vs {base_label}\n{comparison_range_text}", fontsize=16)
        
        # Set axis labels
        ax.set_xlabel("Résolution", fontsize=14)
        ax.set_ylabel("Vitesse relative (%)", fontsize=14)
        
        # Add grid lines
        ax.grid(True, linestyle='--', alpha=0.3)
        
        # Set x-axis range to exactly match the comparison range (no padding)
        ax.set_xlim(min(interp_resolutions), max(interp_resolutions))
        
        # Set y-axis to start from 0 and end at a reasonable value
        if max(speedups) > 200:
            # If there are speedups > 200%, set the top at 110% of max
            ax.set_ylim(0, max(speedups) * 1.1)
        else:
            # Otherwise cap at 210% for better visualization
            ax.set_ylim(0, 210)
        
        # Improve layout
        fig.tight_layout()
        
        # Save the plot if requested
        if output_file:
            output_dir = os.path.dirname(output_file)
            if output_dir and not os.path.exists(output_dir):
                os.makedirs(output_dir, exist_ok=True)
            plt.savefig(output_file, dpi=300)
            print(f"Speedup comparison chart saved to {output_file}")
        
        # Show the plot
        plt.show()
        
        # Optional: Print the interpolated data as a table (only valid data)
        print("\nInterpolated comparison data (within overlapping range):")
        print("Resolution | Base Time (s) | Comparison Time (s) | Speedup (%)")
        print("-" * 65)
        for res, btime, ctime, spd in zip(interp_resolutions, base_times, comp_times, speedups):
            print(f"{res:>9.0f} | {btime:>12.3f} | {ctime:>18.3f} | {spd:>10.1f}")
        
    except Exception as e:
        print(f"Error comparing benchmark files: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Plot speedup percentage between two benchmark CSV files')
    parser.add_argument('--base', required=True, help='Baseline CSV file')
    parser.add_argument('--comparison', required=True, help='Comparison CSV file')
    parser.add_argument('--output', '-o', help='Output file path for the plot')
    parser.add_argument('--title', '-t', help='Title for the plot')
    parser.add_argument('--points', '-p', type=int, default=50, help='Number of interpolation points')
    
    args = parser.parse_args()
    
    plot_speedup_comparison(args.base, args.comparison, args.output, args.title, args.points)