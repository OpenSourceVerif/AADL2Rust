#ifndef __PRODUCER_CONSUMER_H__
#define __PRODUCER_CONSUMER_H__

#include <stdio.h>

typedef int test__ba__backend__alpha_type;

extern int nb_call_of_compute_spg;
void compute_spg(test__ba__backend__alpha_type *a_data_out);
void print_spg(test__ba__backend__alpha_type a_data_in);
void print_spg1(test__ba__backend__alpha_type a_data_in);
void print_thread_begin_execution(int thread_index);

#endif /* __PRODUCER_CONSUMER_H__ */