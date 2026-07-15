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
        <div
            v-for="(ing, i) in ingredients"
            :key="i"
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
        <div v-if="ingredients.length === 0" class="text-base text-base-content/40 text-center py-4">
            还没有选择菜谱
        </div>
    </div>
</template>
