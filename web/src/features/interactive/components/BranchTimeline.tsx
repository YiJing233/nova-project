import { useState } from 'react'
import { ChevronDown, ChevronUp, GitBranch } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import type { BranchSummary, Snapshot } from '../types'

interface BranchTimelineProps {
  snapshot: Snapshot | null
  branches: BranchSummary[]
  currentBranchId: string
  onSwitchBranch: (branchId: string) => void
  onCreateBranch: (turnId: string) => void
}

export function BranchTimeline({ snapshot, branches, currentBranchId, onSwitchBranch, onCreateBranch }: BranchTimelineProps) {
  const [expanded, setExpanded] = useState(false)

  return (
    <div className={`${expanded ? 'h-[116px]' : 'h-[52px]'} border-t border-[#2f3540] bg-[#14171c] px-4 py-3 transition-[height]`}>
      <div className="flex items-center justify-between gap-3 text-xs text-[#858b96]">
        <button type="button" className="flex items-center gap-1.5 font-medium text-[#8f98a8] hover:text-[#dbe3ef]" onClick={() => setExpanded(!expanded)}>
          <GitBranch className="h-3.5 w-3.5 text-[#7fb7e8]" />
          剧情时间线 / 分支树
          {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronUp className="h-3.5 w-3.5" />}
        </button>
        <div className="flex min-w-0 flex-1 items-center justify-end gap-2">
          <span className="truncate text-[#737d8d]">{snapshot?.turns?.length || 0} 个回合</span>
          <div className="flex max-w-[55%] gap-2 overflow-hidden">
            {branches.map((branch) => (
              <Button key={branch.id} variant={branch.id === currentBranchId ? 'default' : 'outline'} size="xs" className={branch.id === currentBranchId ? 'bg-[#2d6fb8] hover:bg-[#347dca]' : 'border-[#343b47] bg-[#20242b] text-[#aab2c0] hover:bg-[#252831]'} onClick={() => onSwitchBranch(branch.id)}>
                {branch.title || branch.id}
              </Button>
            ))}
          </div>
        </div>
      </div>
      {expanded && (
        <ScrollArea className="mt-4 w-full">
          <div className="flex items-center gap-2 pb-1">
            {(snapshot?.turns || []).map((turn, index) => (
              <button key={turn.id} className="group flex items-center gap-2" onClick={() => onCreateBranch(turn.id)} title="从此分叉">
                {index > 0 && <span className="h-px w-12 bg-[#3a465a]" />}
                <Badge className="h-5 w-5 rounded-full bg-[#2d6fb8] p-0 text-[10px] group-hover:ring-2 group-hover:ring-[#2d6fb8]/40">{index + 1}</Badge>
              </button>
            ))}
            {!snapshot?.turns?.length && <span className="text-xs text-[#858b96]">还没有回合，输入第一句话开始。</span>}
          </div>
        </ScrollArea>
      )}
    </div>
  )
}
