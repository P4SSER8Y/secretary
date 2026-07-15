<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
    totalSteps: number
}>()

const current = ref(0)

defineExpose({ current })

watch(() => props.totalSteps, () => {
    current.value = 0
})

function prev() {
    if (current.value > 0) current.value--
}

function next() {
    if (current.value < props.totalSteps - 1) current.value++
}
</script>

<template>
    <div v-if="totalSteps > 0" class="flex items-center gap-2 py-2">
        <button class="btn btn-sm btn-ghost" @click="prev" :disabled="current === 0">◀</button>
        <progress class="progress progress-accent flex-1" :value="current + 1" :max="totalSteps"></progress>
        <span class="text-base whitespace-nowrap">{{ current + 1 }} / {{ totalSteps }}</span>
        <button class="btn btn-sm btn-ghost" @click="next" :disabled="current >= totalSteps - 1">▶</button>
    </div>
</template>
