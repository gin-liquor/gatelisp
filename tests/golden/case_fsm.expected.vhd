library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_case_fsm is
  port (
    gl_p0_clk : in std_logic;
    gl_p1_reset : in std_logic;
    gl_p2_start : in std_logic;
    gl_p3_state_value : out unsigned(1 downto 0);
    gl_p4_busy : out std_logic;
    gl_p5_done : out std_logic
  );
end entity gl_m0_case_fsm;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_case_fsm is
  signal gl_s3_state_value : unsigned(1 downto 0);
  signal gl_s4_busy : std_logic;
  signal gl_s5_done : std_logic;
  signal gl_s6_state : unsigned(1 downto 0) := resize(unsigned'(x"0000000000000000"), 2);
  signal gl_s7_busy_reg : std_logic := '0';
  signal gl_s8_done_reg : std_logic := '0';
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
    gl_s3_state_value <= gl_s6_state;
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s4_busy <= gl_s7_busy_reg;
  end process gl_comb_1;
  gl_comb_2 : process(all)
  begin
    gl_s5_done <= gl_s8_done_reg;
  end process gl_comb_2;
  gl_seq_0 : process(gl_p0_clk)
    variable gl_tmp_0 : unsigned(1 downto 0);
    variable gl_tmp_1 : std_logic;
    variable gl_tmp_2 : unsigned(1 downto 0);
  begin
    if falling_edge(gl_p0_clk) then
      if (gl_p1_reset = '1') then
        gl_s6_state <= resize(unsigned'(x"0000000000000000"), 2);
        gl_s7_busy_reg <= '0';
        gl_s8_done_reg <= '0';
      else
        gl_tmp_0 := gl_s6_state;
        if (gl_tmp_0 = resize(unsigned'(x"0000000000000000"), 2)) then
          gl_tmp_1 := gl_p2_start;
          if (gl_tmp_1 = '0') then
            gl_tmp_2 := resize(unsigned'(x"0000000000000000"), 2);
          else
            if (gl_tmp_1 = '1') then
              gl_tmp_2 := resize(unsigned'(x"0000000000000001"), 2);
            else
              gl_tmp_2 := resize(unsigned'(x"0000000000000000"), 2);
            end if;
          end if;
          gl_s6_state <= gl_tmp_2;
          gl_s7_busy_reg <= '0';
          gl_s8_done_reg <= '0';
        else
          if (gl_tmp_0 = resize(unsigned'(x"0000000000000001"), 2)) then
            gl_s6_state <= resize(unsigned'(x"0000000000000002"), 2);
            gl_s7_busy_reg <= '1';
            gl_s8_done_reg <= '0';
          else
            gl_s6_state <= resize(unsigned'(x"0000000000000000"), 2);
            gl_s7_busy_reg <= '0';
            gl_s8_done_reg <= '1';
          end if;
        end if;
      end if;
    end if;
  end process gl_seq_0;
  gl_p3_state_value <= gl_s3_state_value;
  gl_p4_busy <= gl_s4_busy;
  gl_p5_done <= gl_s5_done;
end architecture rtl;
