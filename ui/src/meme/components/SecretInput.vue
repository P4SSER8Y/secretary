<script setup lang="ts">
import { onMounted, ref } from 'vue';

const props = withDefaults(
    defineProps<{
        modelValue: string;
        placeholder?: string;
        autofocus?: boolean;
    }>(),
    { placeholder: 'key', autofocus: false }
);

const emits = defineEmits<{ 'update:modelValue': [value: string] }>();
const el = ref<HTMLInputElement | null>(null);

onMounted(() => {
    if (props.autofocus) {
        el.value?.focus();
    }
});

function on_input(e: Event) {
    emits('update:modelValue', (e.target as HTMLInputElement).value);
}
</script>

<template>
    <!-- type=search + 字体遮蔽：避开密码管理器对 text/password 字段的识别 -->
    <input
        ref="el"
        :value="props.modelValue"
        type="search"
        class="input input-bordered w-full secret-input"
        :placeholder="props.placeholder"
        :aria-label="props.placeholder"
        name="key"
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        spellcheck="false"
        data-1p-ignore
        data-bwignore
        data-lpignore="true"
        data-protonpass-ignore="true"
        data-dashlane-ignore="true"
        data-form-type="other"
        @input="on_input"
    />
</template>

<style scoped>
@font-face {
    font-family: 'secret-disc';
    src: url('/fonts/text-security-disc-compat.woff2') format('woff2');
    font-display: block;
}

.secret-input {
    font-family: 'secret-disc', monospace;
}

.secret-input::placeholder {
    font-family: system-ui, -apple-system, sans-serif;
}

.secret-input::-webkit-search-decoration,
.secret-input::-webkit-search-cancel-button,
.secret-input::-webkit-search-results-button,
.secret-input::-webkit-search-results-decoration {
    display: none;
}
</style>
