import { Activity, MapPin, Sparkles, UserRoundCheck } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'
import type { Snapshot } from '../types'

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

export function SnapshotPanel({ snapshot }: { snapshot: Snapshot | null }) {
  const onStage = asArray(snapshot?.state?.on_stage)
  const events = asArray(snapshot?.state?.events)
  const state = snapshot?.state || {}
  const characters = snapshot?.state?.characters && typeof snapshot.state.characters === 'object'
    ? Object.entries(snapshot.state.characters as Record<string, unknown>)
    : []
  const location = pickString(state, ['location', 'place', 'scene', '地点'])
  const time = pickString(state, ['time', 'moment', '时间'])
  const pov = pickString(state, ['pov', 'viewpoint', '视角'])

  return (
    <aside className="flex h-full w-[336px] shrink-0 flex-col border-l border-[#2f3540] bg-[#1b1e24] p-4">
      <div className="mb-3 flex h-8 items-center justify-between">
        <div>
          <h2 className="text-sm font-semibold text-[#e0e4ec]">场景记忆</h2>
          <div className="text-[11px] text-[#7f8898]">当前回合的实时上下文</div>
        </div>
        <Badge variant="outline" className="border-[#3a414d] bg-[#252a33] text-[#8d96a7]">{snapshot?.branch_id || 'main'}</Badge>
      </div>
      <ScrollArea className="min-h-0 flex-1 pr-1">
        <section className="mb-3 rounded-lg border border-[#343b47] bg-[#111318] p-3">
          <div className="mb-3 flex items-center gap-2 text-xs font-semibold text-[#7fb7e8]">
            <MapPin className="h-3.5 w-3.5" />
            当前场景
          </div>
          <div className="grid grid-cols-3 gap-2">
            <SnapshotMetric label="地点" value={location || '未记录'} />
            <SnapshotMetric label="时间" value={time || '未记录'} />
            <SnapshotMetric label="视角" value={pov || '未记录'} />
          </div>
        </section>

        <section className="mb-3 rounded-lg border border-[#343b47] bg-[#111318] p-3">
          <div className="mb-3 flex items-center gap-2 text-xs font-semibold text-[#7fb7e8]">
            <UserRoundCheck className="h-3.5 w-3.5" />
            在场角色
          </div>
          <div className="flex flex-wrap gap-1.5 text-sm text-[#a8adb7]">
            {onStage.length ? onStage.map((name) => <Badge key={String(name)} className="bg-[#263646] text-[#d6e9ff]" variant="secondary">{String(name)}</Badge>) : '暂无在场角色'}
          </div>
        </section>

        <section className="mb-3 rounded-lg border border-[#343b47] bg-[#111318] p-3">
          <div className="mb-3 flex items-center gap-2 text-xs font-semibold text-[#7fb7e8]">
            <Activity className="h-3.5 w-3.5" />
            角色状态
          </div>
          <div className="space-y-2 text-xs text-[#a8adb7]">
            {characters.length ? characters.map(([name, state]) => (
              <div key={name} className="rounded-md border border-[#303743] bg-[#191d24] p-2">
                <div className="mb-1 font-medium text-[#d6dbe5]">{name}</div>
                <pre className="whitespace-pre-wrap text-[#9da6b6]">{formatState(state)}</pre>
              </div>
            )) : '暂无角色状态'}
          </div>
        </section>

        <section className="rounded-lg border border-[#343b47] bg-[#111318] p-3">
          <div className="mb-3 flex items-center gap-2 text-xs font-semibold text-[#7fb7e8]">
            <Sparkles className="h-3.5 w-3.5" />
            关键事件
          </div>
          <div className="space-y-2 text-xs text-[#a8adb7]">
            {events.length ? events.map((event, index) => <pre key={index} className="whitespace-pre-wrap rounded-md border border-[#303743] bg-[#191d24] p-2 text-[#9da6b6]">{formatState(event)}</pre>) : '暂无关键事件'}
          </div>
        </section>
      </ScrollArea>
    </aside>
  )
}

function SnapshotMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-md border border-[#303743] bg-[#191d24] px-2 py-2">
      <div className="text-[10px] text-[#747f91]">{label}</div>
      <div className="truncate text-xs font-medium text-[#c8d0dd]" title={value}>{value}</div>
    </div>
  )
}

function pickString(source: Record<string, unknown>, keys: string[]) {
  for (const key of keys) {
    const value = source[key]
    if (typeof value === 'string' && value.trim()) return value.trim()
  }
  return ''
}

function formatState(value: unknown) {
  if (typeof value === 'string') return value
  return JSON.stringify(value, null, 2)
}
