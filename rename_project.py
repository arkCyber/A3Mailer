#!/usr/bin/env python3
"""
A3Mailer Project Renaming Script
This script systematically renames all Stalwart references to A3Mailer
"""

import os
import re
import glob
from pathlib import Path

def update_file_content(file_path, replacements):
    """Update file content with the given replacements"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        original_content = content
        
        # Apply all replacements
        for old_text, new_text in replacements.items():
            content = content.replace(old_text, new_text)
        
        # Only write if content changed
        if content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"✅ Updated: {file_path}")
            return True
        return False
    except Exception as e:
        print(f"❌ Error updating {file_path}: {e}")
        return False

def main():
    print("🚀 Starting A3Mailer project renaming...")
    
    # Define replacements
    replacements = {
        # Copyright and licensing
        "SPDX-FileCopyrightText: 2024 A3Mailer Project": "SPDX-FileCopyrightText: 2024 A3Mailer Project",
        
        # Project names and descriptions
        "A3Mailer DAV Server": "A3Mailer DAV Server",
        "A3Mailer Mail Server": "A3Mailer Mail Server", 
        "A3Mailer SMTP Server": "A3Mailer SMTP Server",
        "A3Mailer IMAP Server": "A3Mailer IMAP Server",
        "A3Mailer HTTP Server": "A3Mailer HTTP Server",
        "A3Mailer Server": "A3Mailer Server",
        "A3Mailer High-Performance Server": "A3Mailer High-Performance Server",
        
        # Documentation and comments
        "A3Mailer implementation": "A3Mailer implementation",
        "A3Mailer protocol": "A3Mailer protocol",
        "A3Mailer backend": "A3Mailer backend",
        "A3Mailer frontend": "A3Mailer frontend",
        "A3Mailer codebase": "A3Mailer codebase",
        "A3Mailer project": "A3Mailer project",
        "A3Mailer Team": "A3Mailer Team",
        
        # Directory and file names
        "a3mailer-server": "a3mailer-server",
        "a3mailer-main": "a3mailer-main",
        
        # URN and identifiers (keep some technical references)
        "urn:a3mailer:": "urn:a3mailer:",
        
        # Configuration and setup
        "A3Mailer configuration": "A3Mailer configuration",
        "A3Mailer setup": "A3Mailer setup",
        "A3Mailer deployment": "A3Mailer deployment",
    }
    
    # File patterns to update
    file_patterns = [
        "**/*.rs",
        "**/*.toml", 
        "**/*.md",
        "**/*.yml",
        "**/*.yaml",
        "**/*.json",
        "**/*.txt",
        "**/*.sh",
        "**/*.py",
    ]
    
    updated_files = 0
    total_files = 0
    
    # Process each file pattern
    for pattern in file_patterns:
        for file_path in glob.glob(pattern, recursive=True):
            # Skip hidden files and directories
            if any(part.startswith('.') for part in Path(file_path).parts):
                continue
                
            # Skip binary files and build artifacts
            if any(skip in file_path for skip in ['target/', 'node_modules/', '.git/', '__pycache__/']):
                continue
                
            total_files += 1
            if update_file_content(file_path, replacements):
                updated_files += 1
    
    print(f"\n🎉 Project renaming completed!")
    print(f"📊 Updated {updated_files} out of {total_files} files")
    print(f"🏷️  Project successfully renamed from Stalwart to A3Mailer")

if __name__ == "__main__":
    main()
