library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_enum_fsm is
  port (
    gl_p0_clk : in std_logic;
    gl_p1_reset : in std_logic;
    gl_p2_start : in std_logic;
    gl_p3_state_out : out unsigned(1 downto 0);
    gl_p4_debug : out unsigned(1 downto 0)
  );
end entity gl_m0_enum_fsm;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_enum_fsm is
  constant gl_enum_state_idle : unsigned(1 downto 0) := to_unsigned(0, 2);
  constant gl_enum_state_run : unsigned(1 downto 0) := to_unsigned(1, 2);
  constant gl_enum_state_done : unsigned(1 downto 0) := to_unsigned(2, 2);
  constant gl_enum_state_fault : unsigned(1 downto 0) := to_unsigned(3, 2);
  signal gl_s3_state_out : unsigned(1 downto 0);
  signal gl_s4_debug : unsigned(1 downto 0);
  signal gl_s5_state : unsigned(1 downto 0) := gl_enum_state_idle;
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_comb_0 : process(all)
  begin
    gl_s3_state_out <= gl_s5_state;
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s4_debug <= gl_s5_state;
  end process gl_comb_1;
  gl_seq_0 : process(gl_p0_clk)
    variable gl_tmp_0 : unsigned(1 downto 0);
    variable gl_tmp_1 : std_logic;
    variable gl_tmp_2 : unsigned(1 downto 0);
  begin
    if rising_edge(gl_p0_clk) then
      if (gl_p1_reset = '1') then
        gl_s5_state <= gl_enum_state_idle;
      else
        gl_tmp_0 := gl_s5_state;
        if (gl_tmp_0 = gl_enum_state_idle) then
          gl_tmp_1 := gl_p2_start;
          if (gl_tmp_1 = '0') then
            gl_tmp_2 := gl_enum_state_idle;
          else
            if (gl_tmp_1 = '1') then
              gl_tmp_2 := gl_enum_state_run;
            else
              gl_tmp_2 := gl_enum_state_idle;
            end if;
          end if;
          gl_s5_state <= gl_tmp_2;
        else
          if (gl_tmp_0 = gl_enum_state_run) then
            gl_s5_state <= gl_enum_state_done;
          else
            if (gl_tmp_0 = gl_enum_state_done) then
              gl_s5_state <= gl_enum_state_idle;
            else
              gl_s5_state <= gl_enum_state_fault;
            end if;
          end if;
        end if;
      end if;
    end if;
  end process gl_seq_0;
  gl_p3_state_out <= gl_s3_state_out;
  gl_p4_debug <= gl_s4_debug;
end architecture rtl;
