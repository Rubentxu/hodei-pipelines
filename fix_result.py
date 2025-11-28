import os
import re

def fix_result_types(file_path):
    try:
        with open(file_path, 'r') as f:
            content = f.read()
        
        # Fix Result<T, E> -> Result<T>
        original = content
        content = re.sub(r'Result<([^,>]+),\s*[^)>]+>', r'Result<\1>', content)
        
        if content != original:
            with open(file_path, 'w') as f:
                f.write(content)
            return True
    except Exception as e:
        print(f"Error processing {file_path}: {e}")
    return False

# Process all Rust files in modules/src
fixed = 0
for root, dirs, files in os.walk('crates/modules/src'):
    for file in files:
        if file.endswith('.rs'):
            if fix_result_types(os.path.join(root, file)):
                fixed += 1

print(f"Fixed Result types in {fixed} files")
