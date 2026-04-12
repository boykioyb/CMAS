<script setup lang="ts">
import { Dialog, DialogPanel, DialogTitle, TransitionRoot, TransitionChild } from '@headlessui/vue'
import { X } from 'lucide-vue-next'

defineProps<{
  open: boolean
  title?: string
}>()

const emit = defineEmits<{
  close: []
}>()
</script>

<template>
  <TransitionRoot :show="open" as="template">
    <Dialog @close="emit('close')" class="relative z-50">
      <TransitionChild
        enter="ease-out duration-200" enter-from="opacity-0" enter-to="opacity-100"
        leave="ease-in duration-150" leave-from="opacity-100" leave-to="opacity-0"
      >
        <div class="fixed inset-0 bg-black/30 backdrop-blur-sm" />
      </TransitionChild>

      <div class="fixed inset-0 flex items-center justify-center p-4">
        <TransitionChild
          enter="ease-out duration-200" enter-from="opacity-0 scale-95" enter-to="opacity-100 scale-100"
          leave="ease-in duration-150" leave-from="opacity-100 scale-100" leave-to="opacity-0 scale-95"
        >
          <DialogPanel class="bg-white dark:bg-gray-800 rounded-2xl shadow-xl w-full max-w-md mx-auto border border-gray-200 dark:border-gray-700 overflow-hidden">
            <div v-if="title" class="flex items-center justify-between px-6 pt-5 pb-0">
              <DialogTitle class="text-lg font-semibold text-gray-900 dark:text-white">{{ title }}</DialogTitle>
              <button @click="emit('close')" class="p-1 rounded-lg text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700">
                <X :size="18" />
              </button>
            </div>
            <div class="p-6">
              <slot />
            </div>
          </DialogPanel>
        </TransitionChild>
      </div>
    </Dialog>
  </TransitionRoot>
</template>
