<script setup lang="ts">
import type { ScaledIngredient } from '../lib/structs'

defineProps<{
    ingredients: ScaledIngredient[]
}>()

function formatAmount(ing: ScaledIngredient): string {
    if (ing.hint && !ing.amount) return ing.hint
    const amt = ing.amount ? `${ing.amount}` : ''
    const unit = ing.unit || ''
    return `${amt}${unit}`.trim() || ing.hint || ''
}
</script>

<template>
    <div class="space-y-2">
        <template v-for="ing in ingredients" :key="ing.name">
            <!-- Compound ingredient group -->
            <div v-if="ing.sub_ingredients && ing.sub_ingredients.length > 0"
                class="rounded bg-base-200 overflow-hidden"
                :class="{ 'opacity-60': ing.optional }"
            >
                <div class="flex items-center gap-2 px-3 py-2 bg-base-300/50">
                    <span class="text-sm font-bold">📦 {{ ing.name }}</span>
                    <span v-if="ing.hint" class="text-xs text-base-content/50">{{ ing.hint }}</span>
                    <span v-if="ing.optional" class="text-xs text-base-content/40">(可选)</span>
                </div>
                <div class="px-2 py-1 space-y-0.5">
                    <div v-for="sub in ing.sub_ingredients" :key="sub.name"
                        class="flex items-start gap-2 p-1.5 pl-4"
                    >
                        <div class="flex-1 min-w-0">
                            <div class="text-sm">{{ sub.name }}</div>
                        </div>
                        <div class="text-sm font-mono whitespace-nowrap text-right text-base-content/60">
                            {{ formatAmount(sub) }}
                        </div>
                    </div>
                </div>
            </div>

            <!-- Flat ingredient -->
            <div v-else
                class="flex items-start gap-2 p-2 rounded bg-base-200"
                :class="{ 'opacity-60': ing.optional }"
            >
                <div class="flex-1 min-w-0">
                    <div class="text-base font-medium">{{ ing.name }}</div>
                    <div
                        v-if="ing.used_in.length > 0"
                        class="text-base text-base-content/40 mt-0.5"
                    >
                        用于：{{ ing.used_in.join('、') }}
                    </div>
                </div>
                <div class="text-base font-mono whitespace-nowrap text-right">
                    {{ formatAmount(ing) }}
                    <span v-if="ing.optional" class="text-base text-base-content/40">(可选)</span>
                </div>
            </div>
        </template>
        <div v-if="ingredients.length === 0" class="text-base text-base-content/40 text-center py-4">
            还没有选择菜谱
        </div>
    </div>
</template>
