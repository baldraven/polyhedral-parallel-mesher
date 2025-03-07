import argparse
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os
import glob
from matplotlib.patches import Rectangle
from matplotlib.ticker import FuncFormatter

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

def generate_label_from_parameters(parameters):
    """Generate a label for the plot legend based on parameters."""
    # Customize this to include the most relevant parameters for your comparison
    label_parts = []
        
    # Extract filename without path for additional context
    if 'filename' in parameters:
        base = os.path.basename(parameters['filename'])
        label_parts.append(os.path.splitext(base)[0])
    
    return ', '.join(label_parts)

def load_csv_data(file_path):
    """Load data from a benchmark CSV file."""
    # Read parameters from comments
    parameters = extract_parameters_from_csv(file_path)
    parameters['filename'] = file_path
    
    # Skip comment lines and read the actual data
    data = pd.read_csv(file_path, comment='#')
    
    return data, parameters

def get_csv_files_from_folder(folder_path):
    """Get all CSV files from a specified folder."""
    if not os.path.isdir(folder_path):
        print(f"Warning: {folder_path} is not a valid directory.")
        return []
    
    csv_pattern = os.path.join(folder_path, "*.csv")
    csv_files = glob.glob(csv_pattern)
    
    if not csv_files:
        print(f"Warning: No CSV files found in {folder_path}")
    else:
        print(f"Found {len(csv_files)} CSV files in {folder_path}")
        
    return csv_files

def plot_benchmark_files(csv_files, output_file=None, plot_title=None):
    """Plot multiple benchmark files with time on x-axis and resolution on y-axis."""
    if not csv_files:
        print("No CSV files to plot.")
        return
        
    # Create figure with two y-axes
    fig, ax1 = plt.subplots(figsize=(14, 10))
    ax2 = ax1.twinx()  # Create a secondary y-axis
    
    # Track min/max values for consistent scaling
    min_time = float('inf')
    max_time = 0
    min_resolution = float('inf')
    max_resolution = 0
    
    # Store all datasets for potential reuse
    all_datasets = []
    
    # Define color cycle for different datasets
    colors = plt.cm.tab10(np.linspace(0, 1, len(csv_files)))
    
    # First pass to determine plot bounds
    for i, file_path in enumerate(csv_files):
        try:
            data, parameters = load_csv_data(file_path)
            
            # Convert milliseconds to seconds
            data['Average Time (s)'] = data['Average Time (ms)'] / 1000
            data['Min Time (s)'] = data['Min Time (ms)'] / 1000
            data['Max Time (s)'] = data['Max Time (ms)'] / 1000
            
            all_datasets.append((data, parameters, colors[i]))
            
            # Update min/max values
            min_resolution = min(min_resolution, data['Resolution'].min())
            max_resolution = max(max_resolution, data['Resolution'].max())
            min_time = min(min_time, data['Average Time (s)'].min())
            max_time = max(max_time, data['Average Time (s)'].max())
            
        except Exception as e:
            print(f"Error loading {file_path}: {e}")
    
    # Second pass to plot with consistent scale
    for data, parameters, color in all_datasets:
        label = generate_label_from_parameters(parameters)
        
        # Plot resolution vs average time with swapped axes
        ax1.plot(data['Average Time (s)'], data['Resolution'], 
                 marker='o', label=label, color=color)
        
        # Add horizontal error bars for time range at each resolution point
        for _, row in data.iterrows():
            res = row['Resolution']
            min_time_point = row['Min Time (s)']
            max_time_point = row['Max Time (s)']
            avg_time = row['Average Time (s)']
            
            # Horizontal error bars for time range
            ax1.errorbar(
                avg_time, res, 
                xerr=[[avg_time - min_time_point], [max_time_point - avg_time]], 
                fmt='none', ecolor=color, alpha=0.3
            )
    
    # Set chart title and labels
    if plot_title:
        plt.title(plot_title, fontsize=16)
    else:
        plt.title("Benchmark Comparison", fontsize=16)
    
    # Set axis labels
    ax1.set_xlabel("Temps (secondes)", fontsize=14)
    ax1.set_ylabel("Résolution", fontsize=14)
    
    # Set up secondary y-axis for "number of points" (resolution²/100)
    def resolution_to_points(y):
        return y * y / 100
    
    def points_to_resolution(y):
        return np.sqrt(y * 100)
    
    # Format the secondary y-axis tick labels
    def format_points(y, pos):
        points = resolution_to_points(y)
        if points >= 1e6:
            return f"{points/1e6:.1f}M"
        elif points >= 1e3:
            return f"{points/1e3:.1f}K"
        else:
            return f"{points:.0f}"
    
    # Set up the secondary y-axis with the same limits
    ax2.set_ylim(ax1.get_ylim())
    ax2.set_ylabel("Nombre de points traités (résolution²/100)", fontsize=14)
    ax2.yaxis.set_major_formatter(FuncFormatter(format_points))
    
    # Add grid lines for both axes
    ax1.grid(True, linestyle='--', alpha=0.7)
    
    # Add legend with smaller font size to accommodate multiple entries
    ax1.legend(fontsize=10, loc='upper left')
    
    # Set x-axis to start from 0
    ax1.set_xlim(0, max_time * 1.05)  # Add 5% padding at the right
    
    # Format x-axis ticks to show seconds with appropriate precision
    def format_seconds(x, pos):
        if x < 0.1:
            return f"{x:.3f}"
        elif x < 1:
            return f"{x:.2f}"
        else:
            return f"{x:.1f}"
    
    ax1.xaxis.set_major_formatter(FuncFormatter(format_seconds))
    
    # Tight layout to maximize chart area
    fig.tight_layout()
    
    # Save the plot if requested
    if output_file:
        output_dir = os.path.dirname(output_file)
        if output_dir and not os.path.exists(output_dir):
            os.makedirs(output_dir, exist_ok=True)
        plt.savefig(output_file, dpi=300)
        print(f"Comparison chart saved to {output_file}")
    
    # Show the plot
    plt.show()

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Plot multiple benchmark CSV files on the same chart')
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--files', nargs='+', help='Individual CSV files to compare')
    group.add_argument('--folder', help='Folder containing CSV files to compare')
    parser.add_argument('--output', '-o', help='Output file path for the combined plot')
    parser.add_argument('--title', '-t', help='Title for the plot')
    
    args = parser.parse_args()
    
    # Get CSV files either from individual files or from folder
    if args.files:
        csv_files = args.files
    else:  # args.folder must be set due to mutually_exclusive_group(required=True)
        csv_files = get_csv_files_from_folder(args.folder)
    
    plot_benchmark_files(csv_files, args.output, args.title)
