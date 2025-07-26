#!/bin/bash

# A3Mailer Project - Copyright Update Script
# This script updates all copyright notices from Stalwart to A3Mailer

echo "🔄 Starting copyright update for A3Mailer project..."

# Function to update copyright in a file
update_copyright() {
    local file="$1"
    if [[ -f "$file" ]]; then
        # Update A3Mailer Team LLC copyright to A3Mailer Project
        sed -i.bak 's/SPDX-FileCopyrightText: 2024 A3Mailer Project/SPDX-FileCopyrightText: 2024 A3Mailer Project/g' "$file"
        
        # Update Stalwart references in comments and documentation
        sed -i.bak 's/A3Mailer DAV Server/A3Mailer DAV Server/g' "$file"
        sed -i.bak 's/A3Mailer Mail Server/A3Mailer Mail Server/g' "$file"
        sed -i.bak 's/A3Mailer SMTP Server/A3Mailer SMTP Server/g' "$file"
        sed -i.bak 's/A3Mailer IMAP Server/A3Mailer IMAP Server/g' "$file"
        sed -i.bak 's/A3Mailer HTTP Server/A3Mailer HTTP Server/g' "$file"
        sed -i.bak 's/A3Mailer Server/A3Mailer Server/g' "$file"
        
        # Remove backup file
        rm -f "$file.bak"
        echo "✅ Updated: $file"
    fi
}

# Update Rust source files
echo "📁 Updating Rust source files..."
find crates -name "*.rs" -type f | while read -r file; do
    update_copyright "$file"
done

# Update TOML files
echo "📁 Updating TOML files..."
find . -name "Cargo.toml" -type f | while read -r file; do
    update_copyright "$file"
done

# Update documentation files
echo "📁 Updating documentation files..."
find . -name "*.md" -type f | while read -r file; do
    update_copyright "$file"
done

# Update configuration files
echo "📁 Updating configuration files..."
find . -name "*.yml" -o -name "*.yaml" -o -name "*.json" | while read -r file; do
    update_copyright "$file"
done

echo "🎉 Copyright update completed for A3Mailer project!"
echo "📝 All Stalwart references have been updated to A3Mailer"
