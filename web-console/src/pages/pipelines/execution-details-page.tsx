import { LiveMetricsPanel } from '@/components/features/observability/live-metrics-panel';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ArrowLeftIcon } from 'lucide-react';
import { useNavigate, useParams } from 'react-router-dom';

export function ExecutionDetailsPage() {
    const { pipelineId, executionId } = useParams();
    const navigate = useNavigate();

    return (
        <div className="p-6 space-y-6">
            <div className="flex items-center gap-4">
                <Button variant="ghost" size="icon" onClick={() => navigate('/pipelines')}>
                    <ArrowLeftIcon className="w-4 h-4" />
                </Button>
                <div>
                    <h1 className="text-2xl font-bold text-nebula-text-primary">Execution Details</h1>
                    <p className="text-sm text-nebula-text-secondary">
                        Pipeline: {pipelineId} | Execution: {executionId}
                    </p>
                </div>
            </div>

            <div className="grid grid-cols-1 gap-6">
                <Card>
                    <CardHeader>
                        <CardTitle>Live Metrics</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <LiveMetricsPanel />
                    </CardContent>
                </Card>
            </div>
        </div>
    );
}
