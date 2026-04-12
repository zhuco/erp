#!/bin/sh
# Health check script for Docker container

# Check if nginx is running
if ! pgrep -x nginx >/dev/null; then
    echo "nginx is not running"
    exit 1
fi

# Check if  index.html exists
if [ ! -f /usr/share/nginx/html/index.html ]; then
    echo "index.html not found"
    exit 1
fi

# Try to fetch health endpoint
if ! wget --quiet --tries=1 --spider http://localhost/health 2>/dev/null; then
    echo "Health endpoint not responding"
    exit 1
fi

echo "Health check passed"
exit 0
