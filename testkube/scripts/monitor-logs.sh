#!/bin/bash
# Real-time log monitoring script for Hodei Pipelines

set -e

NAMESPACE=${NAMESPACE:-hodei-pipelines}
COMPONENT=${COMPONENT:-all}
FOLLOW=${FOLLOW:-true}

show_usage() {
    cat << EEOF
Hodei Pipelines - Real-time Log Monitor

USAGE:
    ./monitor-logs.sh [OPTIONS]

OPTIONS:
    -n, --namespace NS    Kubernetes namespace (default: hodei-pipelines)
    -c, --component COMP  Component to monitor (server|worker|all)
    -t, --tail LINES      Number of lines (default: 50)
    -f, --follow          Follow logs (default: true)
    -h, --help            Show this help

EXAMPLES:
    ./monitor-logs.sh --component server
    ./monitor-logs.sh --component worker --tail 100
EEOF
}

monitor_server() {
    local pod=$(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/component=server -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")

    if [ -z "$pod" ]; then
        echo "❌ No server pod found in namespace $NAMESPACE"
        return 1
    fi

    echo "📡 Monitoring Hodei Server: $pod"
    echo "Namespace: $NAMESPACE"
    echo "Press Ctrl+C to stop"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    if [ "$FOLLOW" = "true" ]; then
        kubectl logs -n $NAMESPACE $pod --tail=$TAIL_LINES -f
    else
        kubectl logs -n $NAMESPACE $pod --tail=$TAIL_LINES
    fi
}

monitor_workers() {
    local pods=$(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/component=worker -o jsonpath='{.items[*].metadata.name}' 2>/dev/null || echo "")

    if [ -z "$pods" ]; then
        echo "❌ No worker pods found in namespace $NAMESPACE"
        return 1
    fi

    local pod_count=$(echo $pods | wc -w)
    echo "📡 Monitoring $pod_count Worker pod(s)"
    echo "Namespace: $NAMESPACE"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    for pod in $pods; do
        echo ""
        echo "🔧 Worker: $pod"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        kubectl logs -n $NAMESPACE $pod --tail=$TAIL_LINES
    done
}

# Parse arguments
TAIL_LINES=50
while [[ $# -gt 0 ]]; do
    case $1 in
        -n|--namespace)
            NAMESPACE="$2"
            shift 2
            ;;
        -c|--component)
            COMPONENT="$2"
            shift 2
            ;;
        -t|--tail)
            TAIL_LINES="$2"
            shift 2
            ;;
        -f|--follow)
            FOLLOW="$2"
            shift 2
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

case $COMPONENT in
    server)
        monitor_server
        ;;
    worker)
        monitor_workers
        ;;
    all)
        echo "Monitoring all components..."
        monitor_server
        echo ""
        monitor_workers
        ;;
    *)
        echo "❌ Unknown component: $COMPONENT"
        show_usage
        exit 1
        ;;
esac
